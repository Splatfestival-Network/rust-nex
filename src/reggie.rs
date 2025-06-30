use std::{env, fs, io};
use std::io::{Error, ErrorKind};
use std::net::{SocketAddrV4, ToSocketAddrs};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use futures::{SinkExt, StreamExt};
use macros::{method_id, rmc_proto, rmc_struct, RmcSerialize};
use once_cell::sync::Lazy;
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use rustls::client::WebPkiServerVerifier;
use rustls::server::WebPkiClientVerifier;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, ServerName, TrustAnchor};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{TlsAcceptor, TlsConnector};
use tokio_rustls::client::TlsStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::Message;
use webpki::anchor_from_trusted_cert;
use rust_nex::common::setup;
use crate::define_rmc_proto;
use crate::nex::account::Account;
use crate::rmc::protocols::{new_rmc_gateway_connection, OnlyRemote, RmcCallable, RmcConnection};
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

pub async fn establish_tls_connection_to(address: &str, server_name: &str) -> TlsStream<TcpStream>{
    let connector = get_configured_tls_connector().await;

    let stream = TcpStream::connect((address, 80u16).to_socket_addrs().unwrap().next().unwrap()).await.unwrap();

    let stream = connector.connect(ServerName::try_from(server_name.to_owned()).unwrap(), stream).await
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


pub struct WebStreamSocket<T: AsyncRead + AsyncWrite + Unpin> {
    socket: WebSocketStream<T>,
    incoming_buffer: Vec<u8>,
    finished_reading: bool,
}

impl<T: AsyncRead + AsyncWrite + Unpin> WebStreamSocket<T> {
    pub fn new(socket: WebSocketStream<T>) -> Self{
        Self{
            incoming_buffer: Default::default(),
            socket,
            finished_reading: false,
        }
    }
}

impl<T: AsyncRead + AsyncWrite + Unpin> AsyncWrite for WebStreamSocket<T> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, Error>> {
        let this = &mut self.get_mut().socket;

        let msg = Message::binary(buf.to_vec());

        match this.poll_ready_unpin(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(Err(e)) => return Poll::Ready(Err(Error::new(ErrorKind::Other, e))),
            Poll::Ready(Ok(())) => {
                // continue on
            }
        }

        let Err(e) = this.start_send_unpin(msg) else {
            return Poll::Ready(Ok(buf.len()));
        };


        Poll::Ready(Err(Error::new(ErrorKind::Other, e)))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let this = &mut self.get_mut().socket;

        match this.poll_flush_unpin(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Err(Error::new(ErrorKind::Other, e))),
            Poll::Ready(Ok(())) => Poll::Ready(Ok(()))
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let this = &mut self.get_mut().socket;

        match this.poll_close_unpin(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Err(Error::new(ErrorKind::Other, e))),
            Poll::Ready(Ok(())) => Poll::Ready(Ok(()))
        }
    }
}

impl<T: AsyncRead + AsyncWrite + Unpin> AsyncRead for WebStreamSocket<T> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let Self {
            incoming_buffer,
            socket,
            finished_reading
        } = &mut self.get_mut();

        if !*finished_reading {
            match socket.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(msg))) => {
                    let Message::Binary(data) = msg else {
                        return Poll::Ready(Err(Error::new(ErrorKind::InvalidData, "got non binary data when trying to emulate stream")));
                    };

                    incoming_buffer.extend_from_slice(&data);
                }
                Poll::Ready(Some(Err(e))) if incoming_buffer.is_empty() => {
                    return Poll::Ready(Err(Error::new(ErrorKind::Other, e)));
                }
                Poll::Ready(None) if incoming_buffer.is_empty() => {
                    *finished_reading = true;
                }
                Poll::Pending if incoming_buffer.is_empty() => {
                    return Poll::Pending
                }
                _ => {}
            }
        }



        if !incoming_buffer.is_empty(){
            let read_ammount = buf.remaining();

            let ammount_taken = read_ammount.min(incoming_buffer.len());

            buf.put_slice(&incoming_buffer[0..ammount_taken]);

            *incoming_buffer = (&incoming_buffer.get(ammount_taken..).unwrap_or(&[])).to_vec();
        }

        Poll::Ready(Ok(()))


        /*if buf.remaining() == 0{


            return Poll::Ready(Ok(()));
        }

        match socket.poll_next_unpin(cx) {
            Poll::Ready(Some(Ok(msg))) => {
                let Message::Binary(data) = msg else {
                    return Poll::Ready(Err(Error::new(ErrorKind::InvalidData, "got non binary data when trying to emulate stream")));
                };

                if data.len() <= buf.remaining() {
                    // if no data remains there is no reason to store anything
                    buf.put_slice(&data);
                } else {
                    let read_ammount = buf.remaining();

                    let ammount_taken = read_ammount.min(data.len());

                    buf.put_slice(&data[..ammount_taken]);

                    *incoming_buffer = data[ammount_taken..].to_vec();
                }


                Poll::Ready(Ok(()))
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Err(Error::new(ErrorKind::Other, e))),
            // EOF
            Poll::Ready(None) => Poll::Ready(Ok(())),
            Poll::Pending => Poll::Pending
        }*/
    }
}

#[derive(Error, Debug)]
pub enum ConnectError{
    #[error(transparent)]
    Tungstenite(#[from] tungstenite::error::Error),
    #[error(transparent)]
    DataSendError(#[from] io::Error),
}

pub async fn tls_connect_to(url: &str) -> Result<TlsStream<WebStreamSocket<MaybeTlsStream<TcpStream>>>, ConnectError>{
    let (stream, _)= connect_async(format!("ws://{}/", url)).await?;

    let webstreamsocket = WebStreamSocket::new(stream);

    let connector = get_configured_tls_connector().await;

    let connection = connector.connect(ServerName::try_from(url.to_string()).unwrap(), webstreamsocket).await?;

    Ok(connection)
}

pub async fn rmc_connect_to<T: RmcCallable + Sync + Send + 'static, U: RmcSerialize, F>(url: &str, init_data: U, create_func: F) -> Result<Arc<T>, ConnectError>
    where
    F: FnOnce(RmcConnection) -> Arc<T>{
    let mut connection = tls_connect_to(url).await?;

    connection.send_buffer(&init_data.to_data()).await?;

    let rmc = new_rmc_gateway_connection(connection.into(), create_func);
    
    Ok(rmc)
}

#[tokio::test]
async fn test(){
    setup();
    
    let socket = connect_async("ws://192.168.178.120:12345/").await;
    let (stream, resp) = socket.unwrap();

    let mut webstreamsocket = WebStreamSocket::new(stream);

    let connector = get_configured_tls_connector().await;

    let connection = connector.connect(ServerName::try_from("agmp-tv.spfn.net").unwrap(), webstreamsocket).await.unwrap();

    let rmc = new_rmc_gateway_connection(connection.into(), |r| {
        Arc::new(OnlyRemote::<RemoteTestProto>::new(r))
    });

    println!("{:?}", rmc.test().await);
}

#[tokio::test]
async fn test_server(){
    setup();
    
    let socket = TcpListener::bind("192.168.178.120:12345").await.unwrap();

    let acceptor = get_configured_tls_acceptor().await;

    while let Ok((stream, _sock_addr)) = socket.accept().await{
        let websocket = tokio_tungstenite::accept_async(stream).await.unwrap();

        let webstreamsocket = WebStreamSocket::new(websocket);

        let stream = acceptor.accept(webstreamsocket).await.unwrap();

        new_rmc_gateway_connection(stream.into(), |_| {
            Arc::new(
                TestStruct
            )
        });
    }
}



#[rmc_proto(1)]
pub trait ProxyManagement {
    #[method_id(1)]
    async fn update_url(&self, url: String) -> Result<(), ErrorCode>;
}

define_rmc_proto!(
    proto Proxy{
        ProxyManagement
    }
);

#[rmc_proto(2)]
pub trait ControllerManagement {
    #[method_id(1)]
    async fn get_secure_proxy_url(&self) -> Result<String, ErrorCode>;

    #[method_id(2)]
    async fn get_secure_account(&self) -> Result<Account, ErrorCode>;
}

define_rmc_proto!(
    proto Controller{
        ControllerManagement
    }
);

#[derive(RmcSerialize)]
#[repr(u32)]
pub enum ServerCluster{
    Auth = 0,
    Secure = 1
}

#[derive(RmcSerialize)]
#[repr(u32)]
pub enum ServerType{
    Proxy{
        addr: SocketAddrV4,
        cluster: ServerCluster
    } = 1,
    Backend{
        name: String,
        cluster: ServerCluster
    } = 2,
}

