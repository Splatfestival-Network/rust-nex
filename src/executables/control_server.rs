use std::future::Future;
use rust_nex::rmc::protocols::{LocalNoProto, RmcCallable};
use rust_nex::rmc::structures::RmcSerialize;
use std::io::Cursor;
use std::net::{Ipv4Addr, SocketAddrV4};
use macros::rmc_struct;
use rust_nex::common::setup;
use rust_nex::executables::common::{ControllerManagement, LocalController, RemoteProxy, RemoteProxyManagement, ServerCluster, ServerType, KERBEROS_SERVER_PASSWORD};
use rust_nex::prudp::station_url::StationUrl;
use rust_nex::reggie::{get_configured_tls_acceptor, TestStruct, WebStreamSocket};
use rust_nex::rmc::protocols::{new_rmc_gateway_connection, OnlyRemote};
use rust_nex::rmc::response::ErrorCode;
use rust_nex::reggie::UnitPacketRead;
use std::sync::{Arc, Weak};
use log::error;
use once_cell::sync::Lazy;
use rand::random;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::task;
use tungstenite::client;
use rust_nex::nex::account::Account;
use rust_nex::rmc::response::ErrorCode::{Core_Exception, Core_InvalidIndex};
use rust_nex::rmc::protocols::RemoteInstantiatable;
use rust_nex::util::SendingBufferConnection;

pub static AUTH_SERVER_ACCOUNT: Lazy<Account> =
    Lazy::new(|| Account::new(1, "Quazal Authentication", &KERBEROS_SERVER_PASSWORD));
pub static SECURE_SERVER_ACCOUNT: Lazy<Account> =
    Lazy::new(|| Account::new(2, "Quazal Rendez-Vous", &KERBEROS_SERVER_PASSWORD));

#[rmc_struct(Controller)]
struct ServerController {
    insecure_proxies: RwLock<Vec<Weak<Proxy>>>,
    insecure_backend_url: RwLock<String>,
    secure_proxies: RwLock<Vec<Weak<Proxy>>>,
    secure_backend_url: RwLock<String>,
    account: Account
}

impl ServerController{
    async fn update_urls(&self, cluster: ServerCluster){
        let url = match cluster{
            ServerCluster::Auth => {
                self.insecure_backend_url.read().await
            }
            ServerCluster::Secure => {
                self.secure_backend_url.read().await
            }
        }.clone();

        let read_lock = match cluster{
            ServerCluster::Auth => {
                self.insecure_proxies.read().await
            }
            ServerCluster::Secure => {
                self.secure_proxies.read().await
            }
        };

        for proxy in read_lock.iter().filter_map(|v| v.upgrade()){
            if let Err(e) = proxy.proxy.update_url(url.clone()).await {
                error!("error whilest updating proxy url: {:?}", e);
            }
        }
    }
}

struct Proxy{
    proxy: RemoteProxy,
    ip: SocketAddrV4,
    controller: Arc<ServerController>
}

impl RmcCallable for Proxy{
    fn rmc_call(&self, responder: &SendingBufferConnection, protocol_id: u16, method_id: u32, call_id: u32, rest: Vec<u8>) -> impl Future<Output=()> + Send {
        self.controller.rmc_call(responder, protocol_id, method_id, call_id, rest)
    }
}


impl ControllerManagement for ServerController {
    async fn get_secure_proxy_url(&self) -> Result<String, ErrorCode> {
        let proxy = self.secure_proxies.write().await;

        let proxies = proxy.iter().filter_map(|v| v.upgrade());

        let idx: usize = random::<usize>() % proxy.len();
        // do not switch this to using regular array indexing i specifically wrote it like this as
        // to have absolutely now way of panicking, we cant have the control server panicking after
        // all
        let Some(proxy) = proxies.clone().nth(idx).or_else(|| proxies.clone().nth(0)) else {
            return Err(Core_InvalidIndex);
        };

        let station_url = format!(
            "prudps:/PID=2;sid=1;stream=10;type=2;address={};port={};CID=1",
            proxy.ip.ip(), proxy.ip.port()
        );

        Ok(station_url)
    }

    async fn get_secure_account(&self) -> Result<Account, ErrorCode> {
        Ok(self.account.clone())
    }
}



#[tokio::main]
async fn main() {
    setup();

    let socket = TcpListener::bind("0.0.0.0:10003").await.unwrap();

    let acceptor = get_configured_tls_acceptor().await;

    let server_controller = Arc::new(ServerController {
        account: SECURE_SERVER_ACCOUNT.clone(),
        secure_proxies: Default::default(),
        secure_backend_url: Default::default(),
        insecure_backend_url: Default::default(),
        insecure_proxies: Default::default(),
    });

    while let Ok((stream, _sock_addr)) = socket.accept().await {
        let websocket = tokio_tungstenite::accept_async(stream).await.unwrap();

        let stream = WebStreamSocket::new(websocket);
        
        let mut stream = acceptor.accept(stream).await.unwrap();
        let server_controller = server_controller.clone();
        tokio::spawn(async move {
            let server_controller = server_controller;
            let Ok(server_type) = stream.read_buffer().await else {
                error!("failed to read server type");
                return;
            };

            let Ok(server_type) = ServerType::deserialize(&mut Cursor::new(server_type)) else {
                error!("failed to read server type");
                return;
            };

            match server_type {
                ServerType::Proxy{
                    addr,
                    cluster
                } => {

                    let mut write_lock = match cluster{
                        ServerCluster::Auth => {
                            server_controller.insecure_proxies.write().await
                        }
                        ServerCluster::Secure => {
                            server_controller.secure_proxies.write().await
                        }
                    };

                    let server_controller_internal = server_controller.clone();

                    let remo = new_rmc_gateway_connection(stream.into(), move |r|
                        Arc::new(Proxy {
                            proxy: RemoteProxy::new(r),
                            ip: addr,
                            controller: server_controller_internal
                        }));

                    write_lock.push(Arc::downgrade(&remo));

                    let url = match cluster{
                        ServerCluster::Auth => {
                            server_controller.insecure_backend_url.read().await
                        }
                        ServerCluster::Secure => {
                            server_controller.secure_backend_url.read().await
                        }
                    }.clone();

                    if let Err(e) = remo.proxy.update_url(url.clone()).await {
                        error!("error whilest updating proxy url: {:?}", e);
                    }

                }
                ServerType::Backend{
                    name,
                    cluster
                } => {
                    let mut url = match cluster{
                        ServerCluster::Auth => {
                            server_controller.insecure_backend_url.write().await
                        }
                        ServerCluster::Secure => {
                            server_controller.secure_backend_url.write().await
                        }
                    };

                    *url = name;
                    drop(url);

                    server_controller.update_urls(cluster).await;

                    new_rmc_gateway_connection(stream.into(), |_| server_controller);
                }
            }
        });
    }
}
