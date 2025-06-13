use std::{env, fs, io};
use std::sync::Arc;
use macros::{method_id, rmc_proto, rmc_struct};
use once_cell::sync::Lazy;
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use rustls::client::WebPkiServerVerifier;
use rustls::server::WebPkiClientVerifier;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, ServerName, TrustAnchor};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::{TlsAcceptor, TlsConnector};
use tokio_rustls::client::TlsStream;
use webpki::anchor_from_trusted_cert;
use crate::define_rmc_proto;
use crate::endianness::IS_BIG_ENDIAN;
use crate::rmc::response::ErrorCode;
use crate::rmc::structures::RmcSerialize;

pub static SERVER_NAME: Lazy<String> = Lazy::new(|| {
    env::var("REGGIE_SERVER_NAME").expect("no server name specified")
});

pub static SELF_CERT: Lazy<CertificateDer<'static>> = Lazy::new(|| CertificateDer::from(fs::read(&format!("/opt/reggie/certs/{}.crt", SERVER_NAME.as_str())).expect("failed to read self cpub ertificate")));
pub static ROOT_CA: Lazy<CertificateDer<'static>> = Lazy::new(|| CertificateDer::from(fs::read("/opt/reggie/certs/CA.crt").expect("failed to read root certipub ficate")));
pub static SELF_KEY: Lazy<PrivateKeyDer<'static>> = Lazy::new(|| PrivateKeyDer::try_from(fs::read(&format!("/opt/reggie/certs/{}.key", SERVER_NAME.as_str())).expect("failed to read self pub key")).expect("failed to read self key"));
pub static ROOT_TRUST_ANCHOR: Lazy<TrustAnchor<'static>> = Lazy::new(|| anchor_from_trusted_cert(&*ROOT_CA).expect("unable to create root ca trust anchor"));



pub fn get_root_store() -> RootCertStore {
    RootCertStore {
        roots: vec![
            ROOT_TRUST_ANCHOR.clone()
        ],
    }
}

pub fn get_root_cert_verifier() -> RootCertStore {
    RootCertStore {
        roots: vec![
            ROOT_TRUST_ANCHOR.clone()
        ],
    }
}


pub async fn get_configured_tls_acceptor() -> TlsAcceptor{
    let store = get_root_store();

    let cert_verifier = WebPkiClientVerifier::builder(store.into())
        .build()
        .expect("unable to build cert verifier");

    let config = ServerConfig::builder()
        //.with_no_client_auth()
        .with_client_cert_verifier(cert_verifier)
        .with_single_cert(vec![
            SELF_CERT.clone(),
            ROOT_CA.clone()
        ], SELF_KEY.clone_key())
        .expect("unable to create server config");

    TlsAcceptor::from(Arc::new(config))
}

pub async fn get_configured_tls_connector() -> TlsConnector{
    let store = get_root_store();

    let cert_verifier = WebPkiServerVerifier::builder(store.into())
        .build()
        .expect("unable to build cert verifier");

    let config = ClientConfig::builder()
        //.with_root_certificates(get_root_store())
        .with_webpki_verifier(cert_verifier)
        .with_client_auth_cert(vec![
            SELF_CERT.clone(),
            ROOT_CA.clone()
        ], SELF_KEY.clone_key())
        .expect("unable to create client config");


    TlsConnector::from(Arc::new(config))
}

pub trait UnitPacketRead: AsyncRead + Unpin{
    async fn read_buffer(&mut self) -> Result<Vec<u8>, io::Error>{
        let mut len_raw: [u8; 4] = [0; 4];

        self.read_exact(&mut len_raw).await?;

        let len = u32::from_le_bytes(len_raw);

        let mut vec = vec![0u8; len as _];

        self.read_exact(&mut vec).await?;

        Ok(vec)
    }
}

impl<T: AsyncRead + Unpin> UnitPacketRead for T{}
pub trait UnitPacketWrite: AsyncWrite + Unpin{
    async fn send_buffer(&mut self, data: &[u8]) -> Result<(), io::Error> {
        let mut dest_data = Vec::new();

        data.serialize(&mut dest_data).expect("ran out of memory or something");

        self.write_all(&dest_data[..]).await?;

        self.flush().await?;

        Ok(())
    }
}

impl<T: AsyncWrite + Unpin> UnitPacketWrite for T{}

pub async fn establish_tls_connection_to(address: &str, server_name: &'static str) -> TlsStream<TcpStream>{
    let connector = get_configured_tls_connector().await;

    let stream = TcpStream::connect(address).await.unwrap();

    let stream = connector.connect(ServerName::try_from(server_name).unwrap(), stream).await
        .expect("unable to connect via tls");

    stream
}

#[rmc_proto(1)]
pub trait RmcTestProto{
    #[method_id(1)]
    async fn test(&self) -> Result<String, ErrorCode>;
}

define_rmc_proto!(
    proto TestProto{
        RmcTestProto
    }
);

#[rmc_struct(TestProto)]
pub struct TestStruct;

impl RmcTestProto for TestStruct{
    async fn test(&self) -> Result<String, ErrorCode> {
        Ok("heya".into())
    }
}
