use std::net::SocketAddrV4;
use std::sync::Arc;
use std::time::Duration;
use futures::future::Remote;
use log::{error, warn};
use macros::rmc_struct;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::task;
use tokio::time::sleep;
use tokio_rustls::client::TlsStream;
use tokio_tungstenite::MaybeTlsStream;
use rust_nex::common::setup;
use rust_nex::executables::common::{OWN_IP_PRIVATE, OWN_IP_PUBLIC, SERVER_PORT};
use rust_nex::prudp::packet::VirtualPort;
use rust_nex::prudp::router::Router;
use rust_nex::prudp::secure::Secure;
use rust_nex::prudp::unsecure::Unsecure;
use rust_nex::reggie::{establish_tls_connection_to, tls_connect_to, ConnectError, ProxyManagement, RemoteController, WebStreamSocket};
use rust_nex::rmc::response::ErrorCode;
use rust_nex::rnex_proxy_common::ConnectionInitData;
use rust_nex::reggie::ServerCluster::Auth;
use rust_nex::reggie::ServerType::Proxy;
use rust_nex::reggie::UnitPacketWrite;
use rust_nex::rmc::structures::RmcSerialize;
use rust_nex::reggie::UnitPacketRead;
use rust_nex::rmc::protocols::RemoteInstantiatable;
use rust_nex::reggie::LocalProxy;
use rust_nex::reggie::RemoteControllerManagement;


#[rmc_struct(Proxy)]
struct DestinationHolder{
    url: RwLock<String>,
    controller: RemoteController
}

impl ProxyManagement for DestinationHolder{
    async fn update_url(&self, new_url: String) -> Result<(), ErrorCode> {
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
            |r| Arc::new(DestinationHolder{
                url: Default::default(),
                controller: RemoteController::new(r)
            })
        ).await;
    let dest_holder = conn.unwrap();


    let (router_secure, _) = Router::new(SocketAddrV4::new(*OWN_IP_PRIVATE, *SERVER_PORT))
        .await
        .expect("unable to start router");

    let mut socket_secure = router_secure
        .add_socket(VirtualPort::new(1, 10), Secure(
            "6f599f81",
            dest_holder.controller.get_secure_account().await.unwrap()
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