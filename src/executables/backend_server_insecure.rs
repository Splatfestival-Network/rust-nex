use rust_nex::reggie::{RemoteEdgeNodeHolder, UnitPacketRead};
use log::{error, info};
use once_cell::sync::Lazy;
use rustls::client::danger::HandshakeSignatureValid;
use rustls::pki_types::{CertificateDer, TrustAnchor, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::server::{ClientCertVerifierBuilder, WebPkiClientVerifier};
use rustls::{
    DigitallySignedStruct, DistinguishedName, Error, RootCertStore, ServerConfig, ServerConnection,
    SignatureScheme,
};
use rustls_pki_types::PrivateKeyDer;
use rust_nex::common::setup;
use std::borrow::ToOwned;
use std::{env, fs};
use std::io::Cursor;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use macros::{method_id, rmc_proto, rmc_struct};
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpSocket, TcpStream};
use tokio::task;
use tokio_rustls::TlsAcceptor;
use rust_nex::define_rmc_proto;
use rust_nex::executables::common::{OWN_IP_PRIVATE, SECURE_EDGE_NODE_HOLDER, SECURE_SERVER_ACCOUNT, SERVER_PORT};
use rust_nex::nex::auth_handler::AuthHandler;
use rust_nex::reggie::EdgeNodeHolderConnectOption::DontRegister;
use rust_nex::rmc::protocols::{new_rmc_gateway_connection, OnlyRemote};
use rust_nex::rmc::response::ErrorCode;
use rust_nex::rmc::structures::RmcSerialize;
use rust_nex::rnex_proxy_common::ConnectionInitData;
use rust_nex::util::SplittableBufferConnection;

pub static SECURE_PROXY_ADDR: Lazy<Ipv4Addr> = Lazy::new(|| {
    env::var("SECURE_PROXY_ADDR")
        .ok()
        .and_then(|s| s.parse().ok())
        .expect("no secure proxy ip specified")
});

pub static SECURE_PROXY_PORT: Lazy<u16> = Lazy::new(|| {
    env::var("SECURE_PROXY_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10000)
});



#[tokio::main]
async fn main() {
    setup();

    let conn = TcpStream::connect(&*SECURE_EDGE_NODE_HOLDER).await.unwrap();

    let conn: SplittableBufferConnection = conn.into();

    conn.send(DontRegister.to_data()).await;

    let conn = new_rmc_gateway_connection(conn, |r| Arc::new(OnlyRemote::<RemoteEdgeNodeHolder>::new(r)));

    let listen = TcpListener::bind(SocketAddrV4::new(*OWN_IP_PRIVATE, *SERVER_PORT)).await.unwrap();



    while let Ok((mut stream, addr)) = listen.accept().await {
        let buffer = match stream.read_buffer().await{
            Ok(v) => v,
            Err(e) => {
                error!("an error ocurred whilest reading connection data buffer: {:?}", e);
                continue;
            }
        };

        let user_connection_data = ConnectionInitData::deserialize(&mut Cursor::new(buffer));

        let user_connection_data = match user_connection_data{
            Ok(v) => v,
            Err(e) => {
                error!("an error ocurred whilest reading connection data: {:?}", e);
                continue;
            }
        };
        let controller = conn.clone();
        task::spawn(async move {
            info!("connection to secure backend established");
            new_rmc_gateway_connection(stream.into(), |_| {
                Arc::new(AuthHandler {
                    destination_server_acct: &SECURE_SERVER_ACCOUNT,
                    build_name: "branch:origin/project/wup-agmj build:3_8_15_2004_0",
                    control_server: controller
                })
            });
        });

    }
}
