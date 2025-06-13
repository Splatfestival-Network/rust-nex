mod proxy_secure;

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
use splatoon_server_rust::common::setup;
use splatoon_server_rust::prudp::packet::VirtualPort;
use splatoon_server_rust::prudp::router::Router;
use splatoon_server_rust::prudp::unsecure::Unsecure;
use splatoon_server_rust::reggie::{establish_tls_connection_to, UnitPacketRead, UnitPacketWrite};
use splatoon_server_rust::rmc::structures::RmcSerialize;
use splatoon_server_rust::rnex_proxy_common::ConnectionInitData;

static OWN_IP_PRIVATE: Lazy<Ipv4Addr> = Lazy::new(|| {
    env::var("SERVER_IP")
        .ok()
        .and_then(|s| s.parse().ok())
        .expect("no public ip specified")
});

static OWN_IP_PUBLIC: Lazy<String> =
    Lazy::new(|| env::var("SERVER_IP_PUBLIC").unwrap_or(OWN_IP_PRIVATE.to_string()));

static SERVER_PORT: Lazy<u16> = Lazy::new(|| {
    env::var("AUTH_SERVER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10000)
});

static FORWARD_DESTINATION: Lazy<String> =
    Lazy::new(|| env::var("FORWARD_DESTINATION").unwrap_or(OWN_IP_PRIVATE.to_string()));

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
                = establish_tls_connection_to("192.168.178.120:2376", "account.spfn.net").await;

            if let Err(e) = stream.send_buffer(&ConnectionInitData{
                prudpsock_addr: conn.socket_addr
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