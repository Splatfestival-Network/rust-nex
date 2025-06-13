

use std::env;
use std::ffi::CStr;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use bytemuck::{Pod, Zeroable};
use chacha20::{ChaCha20, Key};
use chacha20::cipher::{Iv, KeyIvInit, StreamCipher};
use log::error;
use once_cell::sync::Lazy;
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey, Document};
use rsa::{BigUint, Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use rsa::pkcs1::EncodeRsaPublicKey;
use rsa::pss::BlindedSigningKey;
use rsa::signature::{RandomizedSigner, SignatureEncoding};
use sha2::Sha256;
use tokio::net::TcpSocket;
use tokio::task;
use rust_nex::common::setup;
use rust_nex::executables::common::{OWN_IP_PRIVATE, SERVER_PORT};
use rust_nex::prudp::packet::VirtualPort;
use rust_nex::prudp::router::Router;
use rust_nex::prudp::unsecure::Unsecure;
use rust_nex::reggie::{establish_tls_connection_to, UnitPacketRead, UnitPacketWrite};
use rust_nex::rmc::structures::RmcSerialize;
use rust_nex::rnex_proxy_common::ConnectionInitData;



static FORWARD_DESTINATION: Lazy<String> =
    Lazy::new(|| env::var("FORWARD_DESTINATION").expect("no forward destination given"));
static FORWARD_DESTINATION_NAME: Lazy<String> =
    Lazy::new(|| env::var("FORWARD_DESTINATION_NAME").expect("no forward destination name given"));

static RSA_PRIVKEY: Lazy<RsaPrivateKey> = Lazy::new(|| {
    let path = env::var("RSA_PRIVKEY")
        .expect("RSA_PRIVKEY not set");

    RsaPrivateKey::read_pkcs8_pem_file(&path)
        .expect("unable to read private key")
});

static RSA_PUBKEY: Lazy<RsaPublicKey> = Lazy::new(|| {
    RSA_PRIVKEY.to_public_key()
});

static PUBKEY_ENCODED: Lazy<Document> = Lazy::new(|| {
    RSA_PUBKEY.to_pkcs1_der().expect("unable to convert pubkey to der")
});

static RSA_SIGNKEY: Lazy<BlindedSigningKey<Sha256>> = Lazy::new(||
    BlindedSigningKey::<Sha256>::new(RSA_PRIVKEY.clone())
);

#[tokio::main]
async fn main() {
    setup();

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
                = establish_tls_connection_to(FORWARD_DESTINATION.as_str(), FORWARD_DESTINATION_NAME.as_str()).await;

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
                }
            }
        });
    }
}