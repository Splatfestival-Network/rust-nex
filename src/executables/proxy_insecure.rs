
use rust_nex::reggie::RemoteEdgeNodeHolder;
use std::env;
use std::ffi::CStr;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4};
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
use tokio::net::{TcpSocket, TcpStream};
use tokio::sync::RwLock;
use tokio::task;
use tokio::time::sleep;
use rust_nex::common::setup;
use rust_nex::executables::common::{FORWARD_DESTINATION, OWN_IP_PRIVATE, OWN_IP_PUBLIC, SECURE_EDGE_NODE_HOLDER, SERVER_PORT};
use rust_nex::prudp::packet::VirtualPort;
use rust_nex::prudp::router::Router;
use rust_nex::prudp::station_url::StationUrl;
use rust_nex::prudp::unsecure::Unsecure;
use rust_nex::reggie::{UnitPacketRead, UnitPacketWrite};
use rust_nex::reggie::EdgeNodeHolderConnectOption::{DontRegister, Register};
use rust_nex::rmc::protocols::{new_rmc_gateway_connection, OnlyRemote};
use rust_nex::rmc::response::ErrorCode;
use rust_nex::rmc::structures::RmcSerialize;
use rust_nex::rnex_proxy_common::ConnectionInitData;
use rust_nex::util::SplittableBufferConnection;




#[tokio::main]
async fn main() {
    setup();

    let conn = tokio::net::TcpStream::connect(&*SECURE_EDGE_NODE_HOLDER).await.unwrap();

    let conn: SplittableBufferConnection = conn.into();

    conn.send(Register(SocketAddrV4::new(*OWN_IP_PUBLIC, *SERVER_PORT)).to_data()).await;

    let conn = new_rmc_gateway_connection(conn, |r| Arc::new(OnlyRemote::<RemoteEdgeNodeHolder>::new(r)));

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

        task::spawn(async move {
            let mut stream
                = match TcpStream::connect(*FORWARD_DESTINATION).await {
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