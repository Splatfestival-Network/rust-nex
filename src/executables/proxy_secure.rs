use rust_nex::reggie::RemoteRmcTestProto;
use std::fs;
use std::net::IpAddr;
use std::sync::Arc;
use rustls::ClientConfig;
use rustls_pki_types::ServerName;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio_rustls::{TlsConnector, TlsStream};
use rust_nex::common::setup;
use rust_nex::reggie::{establish_tls_connection_to, get_configured_tls_connector, RemoteTestProto, UnitPacketWrite};
use rust_nex::rmc::protocols::{new_rmc_gateway_connection, OnlyRemote};
use rust_nex::rmc::structures::RmcSerialize;


#[tokio::main]
async fn main(){
    setup();

    let mut stream
        = establish_tls_connection_to("192.168.178.120:2376", "account.spfn.net").await;

    let remo = new_rmc_gateway_connection(stream.into(), |r| Arc::new(OnlyRemote::<RemoteTestProto>::new(r)) );

    println!("{:?}", remo.test().await);
}