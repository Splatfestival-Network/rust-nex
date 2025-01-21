use std::env::current_dir;
use std::{env, fs};
use std::fs::File;
use std::net::{Ipv4Addr, SocketAddrV4};
use chrono::Local;
use log::{info, trace};
use once_cell::sync::Lazy;
use simplelog::{ColorChoice, CombinedLogger, Config, LevelFilter, TerminalMode, TermLogger, WriteLogger};
use crate::prudp::socket::{Socket, SocketImpl};
use crate::prudp::packet::VirtualPort;
use crate::prudp::router::Router;

mod endianness;
mod prudp;

static AUTH_SERVER_PORT: Lazy<u16> = Lazy::new(||{
    env::var("AUTH_SERVER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10000)
});

static OWN_IP: Lazy<Ipv4Addr> = Lazy::new(||{
    env::var("SERVER_IP")
        .ok()
        .and_then(|s| s.parse().ok())
        .expect("no public ip specified")
});

#[tokio::main]
async fn main() {
    CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Info, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
            WriteLogger::new(LevelFilter::max(), Config::default(), {
                fs::create_dir_all("log").unwrap();
                File::create(format!("log/{}.log", Local::now().to_rfc2822())).unwrap()
            })
        ]
    ).unwrap();

    dotenv::dotenv().ok();

    start_servers().await;
}

async fn start_servers(){
    info!("starting auth server on {}:{}", *OWN_IP, *AUTH_SERVER_PORT);

    let auth_server_router =
        Router::new(SocketAddrV4::new(*OWN_IP, *AUTH_SERVER_PORT)).await
            .expect("unable to startauth server");

    info!("setting up endpoints");

    let mut socket =
        Socket::new(
            auth_server_router.clone(),
            VirtualPort::new(1,10),
            "6f599f81"
        ).await.expect("unable to create socket");

    let Some(connection) = socket.accept().await else {
        return;
    };
}
