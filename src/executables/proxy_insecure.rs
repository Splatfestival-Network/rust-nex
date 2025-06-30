
use rust_nex::reggie::{tls_connect_to, LocalProxy};
use std::env;
use std::ffi::CStr;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use bytemuck::{Pod, Zeroable};
use chacha20::{ChaCha20, Key};
use chacha20::cipher::{Iv, KeyIvInit, StreamCipher};
use log::{error, warn};
use macros::rmc_struct;
use once_cell::sync::Lazy;
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey, Document};
use rsa::{BigUint, Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use rsa::pkcs1::EncodeRsaPublicKey;
use rsa::pss::BlindedSigningKey;
use rsa::signature::{RandomizedSigner, SignatureEncoding};
use sha2::Sha256;
use tokio::net::TcpSocket;
use tokio::sync::RwLock;
use tokio::task;
use tokio::time::sleep;
use rust_nex::common::setup;
use rust_nex::executables::common::{OWN_IP_PRIVATE, OWN_IP_PUBLIC, SERVER_PORT};
use rust_nex::prudp::packet::VirtualPort;
use rust_nex::prudp::router::Router;
use rust_nex::prudp::station_url::StationUrl;
use rust_nex::prudp::unsecure::Unsecure;
use rust_nex::reggie::{establish_tls_connection_to, ProxyManagement, UnitPacketRead, UnitPacketWrite};
use rust_nex::reggie::ServerCluster::Auth;
use rust_nex::reggie::ServerType::Proxy;
use rust_nex::rmc::protocols::OnlyRemote;
use rust_nex::rmc::response::ErrorCode;
use rust_nex::rmc::structures::RmcSerialize;
use rust_nex::rnex_proxy_common::ConnectionInitData;



static FORWARD_DESTINATION: Lazy<String> =
    Lazy::new(|| env::var("FORWARD_DESTINATION").expect("no forward destination given"));
static FORWARD_DESTINATION_NAME: Lazy<String> =
    Lazy::new(|| env::var("FORWARD_DESTINATION_NAME").expect("no forward destination name given"));

#[rmc_struct(Proxy)]
#[derive(Default)]
struct DestinationHolder{
    url: RwLock<String>
}

impl ProxyManagement for DestinationHolder{
    async fn update_url(&self, new_url: String) -> Result<(), ErrorCode> {
        println!("updating url");

        let mut url = self.url.write().await;

        *url = new_url;

        Ok(())
    }
}


#[tokio::main]
async fn main() {
    setup();

    let conn =
        rust_nex::reggie::rmc_connect_to(
            "agmp-control.spfn.net",
            Proxy {
                addr: SocketAddrV4::new(*OWN_IP_PUBLIC, *SERVER_PORT),
                cluster: Auth
            },
            |r| Arc::new(DestinationHolder::default())
        ).await;
    let dest_holder = conn.unwrap();


    let (router_secure, _) = Router::new(SocketAddrV4::new(*OWN_IP_PRIVATE, *SERVER_PORT))
        .await
        .expect("unable to start router");

    let mut socket_secure = router_secure
        .add_socket(VirtualPort::new(1, 10), Unsecure(
            "6f599f81"
        ))
        .await
        .expect("unable to add socket");

    // let conn = socket_secure.connect(auth_sockaddr).await.unwrap();

    loop {
        let Some(mut conn) = socket_secure.accept().await else {
            error!("server crashed");
            return;
        };

        let dest_holder = dest_holder.clone();

        task::spawn(async move {
            let dest = dest_holder.url.read().await;

            if *dest == ""{
                warn!("no destination set yet but connection attempted");
                return;
            }

            let mut stream
                = match tls_connect_to(&dest).await {
                Ok(v) => v,
                Err(e) => {
                    error!("unable to connect: {}", e);
                    return;
                }
            };

            if let Err(e) = stream.send_buffer(&ConnectionInitData{
                prudpsock_addr: conn.socket_addr,
                pid: conn.user_id
            }.to_data()).await{
                error!("error connecting to backend: {}", e);
                return;
            };

            loop {
                tokio::select! {
                    data = conn.recv() => {
                        let Some(data) = data else {
                            break;
                        };

                        if let Err(e) = stream.send_buffer(&data[..]).await{
                            error!("error sending data to backend: {}", e);
                            break;
                        }
                    },
                    data = stream.read_buffer() => {
                        let data = match data{
                            Ok(d) => d,
                            Err(e) => {
                                error!("error reveiving data from backend: {}", e);
                                break;
                            }
                        };
                        
                        if conn.send(data).await == None{
                            return;
                        }
                    },
                    _ = sleep(Duration::from_secs(10)) => {
                        conn.send([0,0,0,0,0].to_vec()).await;
                    }
                }
            }
        });
    }
}