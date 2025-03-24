#![allow(dead_code)]
#![warn(missing_docs)]

//! # Splatoon RNEX server
//!
//! This server still includes the code for rnex itself as this is the first rnex server and thus
//! also the first and only current usage of rnex, expect this and rnex to be split into seperate
//! repos soon.

use crate::rmc::protocols::auth::RemoteAuth;
use crate::rmc::protocols::auth::RawAuthInfo;
use crate::rmc::protocols::auth::RawAuth;
use crate::nex::account::Account;
use crate::prudp::packet::VirtualPort;
use crate::prudp::router::Router;
use crate::prudp::sockaddr::PRUDPSockAddr;
use crate::prudp::socket::Unsecure;
use chrono::{Local, SecondsFormat};
use log::info;
use once_cell::sync::Lazy;
use simplelog::{
    ColorChoice, CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode, WriteLogger,
};
use std::fs::File;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::{env, fs};
use std::marker::PhantomData;
use std::ops::{BitAnd, BitOr};
use std::str::FromStr;
use macros::rmc_struct;
use crate::rmc::protocols::auth::Auth;
use crate::rmc::protocols::{new_rmc_gateway_connection, OnlyRemote};
use crate::rmc::response::ErrorCode;
use crate::rmc::structures::any::Any;
use crate::rmc::structures::connection_data::ConnectionData;
use crate::rmc::structures::qresult::QResult;

mod endianness;
mod prudp;
pub mod rmc;
//mod protocols;

mod grpc;
mod kerberos;
mod nex;
mod web;
mod versions;
mod result;

static KERBEROS_SERVER_PASSWORD: Lazy<String> = Lazy::new(|| {
    env::var("AUTH_SERVER_PASSWORD")
        .ok()
        .unwrap_or("password".to_owned())
});

static AUTH_SERVER_ACCOUNT: Lazy<Account> =
    Lazy::new(|| Account::new(1, "Quazal Authentication", &KERBEROS_SERVER_PASSWORD));
static SECURE_SERVER_ACCOUNT: Lazy<Account> =
    Lazy::new(|| Account::new(2, "Quazal Rendez-Vous", &KERBEROS_SERVER_PASSWORD));

static AUTH_SERVER_PORT: Lazy<u16> = Lazy::new(|| {
    env::var("AUTH_SERVER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10000)
});
static SECURE_SERVER_PORT: Lazy<u16> = Lazy::new(|| {
    env::var("SECURE_SERVER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10002)
});

static OWN_IP_PRIVATE: Lazy<Ipv4Addr> = Lazy::new(|| {
    env::var("SERVER_IP")
        .ok()
        .and_then(|s| s.parse().ok())
        .expect("no public ip specified")
});

static OWN_IP_PUBLIC: Lazy<String> =
    Lazy::new(|| env::var("SERVER_IP_PUBLIC").unwrap_or(OWN_IP_PRIVATE.to_string()));

static SECURE_STATION_URL: Lazy<String> = Lazy::new(|| {
    format!(
        "prudps:/PID=2;sid=1;stream=10;type=2;address={};port={};CID=1",
        *OWN_IP_PUBLIC, *SECURE_SERVER_PORT
    )
});

#[tokio::main]
async fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(LevelFilter::max(), Config::default(), {
            fs::create_dir_all("log").unwrap();
            let date = Local::now().to_rfc3339_opts(SecondsFormat::Secs, false);
            // this fixes windows being windows
            let date = date.replace(":", "-");
            let filename = format!("{}.log", date);
            if cfg!(windows) {
                File::create(format!("log\\{}", filename)).unwrap()
            } else {
                File::create(format!("log/{}", filename)).unwrap()
            }
        }),
    ])
    .unwrap();

    dotenv::dotenv().ok();

    start_servers().await;
}
/*

struct AuthServer{
    router: Arc<Router>,
    join_handle: JoinHandle<()>,
    socket: Socket
}

async fn start_auth_server() -> AuthServer{
    info!("starting auth server on {}:{}", *OWN_IP_PRIVATE, *AUTH_SERVER_PORT);

    let (router, join_handle) =
        Router::new(SocketAddrV4::new(*OWN_IP_PRIVATE, *AUTH_SERVER_PORT)).await
            .expect("unable to startauth server");

    info!("setting up endpoints");

    // dont assign it to the name _ as that will make it drop right here and now

    let auth_protocol_config = AuthProtocolConfig{
        secure_server_account: &SECURE_SERVER_ACCOUNT,
        build_name: "branch:origin/project/wup-agmj build:3_8_15_2004_0",
        station_url: &SECURE_STATION_URL
    };

    let rmcserver = RMCProtocolServer::new(Box::new([
        Box::new(auth::bound_protocol(auth_protocol_config))
    ]));

    let socket =
        Socket::new(
            router.clone(),
            VirtualPort::new(1,10),
            "6f599f81",
            Box::new(|_, count|{
                Box::pin(
                    async move {


                        let encryption_pairs = Vec::from_iter((0..=count).map(|_v| {
                            let rc4: Rc4<U5> = Rc4::new_from_slice( "CD&ML".as_bytes()).unwrap();
                            let cypher = Box::new(rc4);
                            let server_cypher: Box<dyn StreamCipher + Send> = cypher;

                            let rc4: Rc4<U5> = Rc4::new_from_slice( "CD&ML".as_bytes()).unwrap();
                            let cypher = Box::new(rc4);
                            let client_cypher: Box<dyn StreamCipher + Send> = cypher;

                            EncryptionPair{
                                recv: client_cypher,
                                send: server_cypher
                            }
                        }));

                        Some((Vec::new(), encryption_pairs, None))
                    }
                )
            }),
            Box::new(move |packet, socket, connection|{
                let rmcserver = rmcserver.clone();
                Box::pin(async move { rmcserver.process_message(packet, socket, connection).await; })
            })
        ).await.expect("unable to create socket");

    AuthServer{
        join_handle,
        router,
        socket,
    }
}

struct SecureServer{
    router: Arc<Router>,
    join_handle: JoinHandle<()>,
    socket: Socket
}

async fn start_secure_server() -> SecureServer{
    info!("starting secure server on {}:{}", *OWN_IP_PRIVATE, *SECURE_SERVER_PORT);

    let (router, join_handle) =
        Router::new(SocketAddrV4::new(*OWN_IP_PRIVATE, *SECURE_SERVER_PORT)).await
            .expect("unable to startauth server");

    info!("setting up endpoints");

    let matchmake_data = Arc::new(RwLock::new(
        MatchmakeData{
            matchmake_sessions: BTreeMap::new()
        }
    ));

    let rmcserver = RMCProtocolServer::new(Box::new([
        Box::new(block_if_maintenance),
        Box::new(protocols::secure::bound_protocol()),
        Box::new(protocols::matchmake::bound_protocol(matchmake_data.clone())),
        Box::new(protocols::matchmake_extension::bound_protocol(matchmake_data)),
        Box::new(protocols::nat_traversal::bound_protocol())
    ]));

    let socket =
        Socket::new(
            router.clone(),
            VirtualPort::new(1,10),
            "6f599f81",
            Box::new(|p, count|{
                Box::pin(
                    async move {
                        let (session_key, pid, check_value) = read_secure_connection_data(&p.payload, &SECURE_SERVER_ACCOUNT)?;

                        let check_value_response = check_value + 1;

                        let data = bytemuck::bytes_of(&check_value_response);

                        let mut response = Vec::new();

                        data.serialize(&mut response).ok()?;

                        let encryption_pairs = generate_secure_encryption_pairs(session_key, count);

                        Some((response, encryption_pairs, Some(
                            ActiveSecureConnectionData{
                                pid,
                                session_key
                            }
                        )))
                    }
                )
            }),
            Box::new(move |packet, socket, connection|{
                let rmcserver = rmcserver.clone();
                Box::pin(async move { rmcserver.process_message(packet, socket, connection).await; })
            })
        ).await.expect("unable to create socket");

    SecureServer{
        join_handle,
        router,
        socket,
    }
}*/

define_rmc_proto!(
    proto AuthClientProtocol{
        Auth
    }
);

impl Auth for AuthClient{
    async fn login(&self, name: String) -> Result<(), ErrorCode> {
        todo!()
    }

    async fn login_ex(&self, name: String, extra_data: Any) -> Result<(QResult, u32, Vec<u8>, ConnectionData, String), ErrorCode> {
        todo!()
    }

    async fn request_ticket(&self, source_pid: u32, destination_pid: u32) -> Result<(QResult, Vec<u8>), ErrorCode> {
        todo!()
    }

    async fn get_pid(&self, username: String) -> Result<u32, ErrorCode> {
        todo!()
    }

    async fn get_name(&self, pid: u32) -> Result<String, ErrorCode> {
        todo!()
    }
}

#[rmc_struct(AuthClientProtocol)]
struct AuthClient {}

async fn start_servers() {
    
    
    //let auth_ip = SocketAddrV4::from_str("157.90.13.221:30039").unwrap();
    let auth_ip = SocketAddrV4::from_str("31.220.75.208:10000").unwrap();
    let auth_port = VirtualPort::new(1, 10);

    let auth_sockaddr = PRUDPSockAddr::new(auth_ip, auth_port);

    let (router_secure, _) = Router::new(SocketAddrV4::new(*OWN_IP_PRIVATE, *SECURE_SERVER_PORT))
        .await
        .expect("unable to start router");

    let mut socket_secure = router_secure
        .add_socket(VirtualPort::new(1, 10), Unsecure("CD&ML"))
        .await
        .expect("unable to add socket");

    let conn = socket_secure.connect(auth_sockaddr).await.unwrap();

    let obj = new_rmc_gateway_connection(conn, OnlyRemote::<RemoteAuthClientProtocol>::new);



    /*
    #[cfg(feature = "auth")]
    let auth_server = start_auth_server().await;
    #[cfg(feature = "secure")]
    let secure_server = start_secure_server().await;
    let web_server = web::start_web().await;

    #[cfg(feature = "auth")]
    auth_server.join_handle.await.expect("auth server crashed");
    #[cfg(feature = "secure")]
    secure_server.join_handle.await.expect("auth server crashed");
    web_server.await.expect("webserver crashed");*/
}