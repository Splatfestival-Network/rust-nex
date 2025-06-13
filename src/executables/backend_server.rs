use log::error;
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
use splatoon_server_rust::common::setup;
use splatoon_server_rust::reggie::{get_configured_tls_acceptor, TestStruct, ROOT_TRUST_ANCHOR, SELF_CERT, SELF_KEY};
use std::borrow::ToOwned;
use std::fs;
use std::io::Cursor;
use std::net::{IpAddr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use macros::{method_id, rmc_proto, rmc_struct};
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpSocket};
use tokio::task;
use tokio_rustls::TlsAcceptor;
use splatoon_server_rust::define_rmc_proto;
use splatoon_server_rust::rmc::protocols::new_rmc_gateway_connection;
use splatoon_server_rust::rmc::response::ErrorCode;
use splatoon_server_rust::rmc::structures::RmcSerialize;




#[tokio::main]
async fn main() {
    setup();

    let acceptor = get_configured_tls_acceptor().await;

    let listen = TcpListener::bind("192.168.178.120:2376").await.unwrap();

    while let Ok((stream, addr)) = listen.accept().await {
        let mut stream = match acceptor.accept(stream).await {
            Ok(v) => v,
            Err(e) => {
                error!("an error ocurred whilest accepting tls connection: {:?}", e);
                continue;
            }
        };

        task::spawn(async move {
            new_rmc_gateway_connection(stream.into(), |_| {
                Arc::new(TestStruct)
            });

            println!("lost connection lol");
        });

    }
}
