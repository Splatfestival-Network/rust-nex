use std::env::current_dir;
use std::{env, fs};
use std::fs::File;
use std::net::{Ipv4Addr, SocketAddrV4};
use chrono::Local;
use log::info;
use once_cell::sync::Lazy;
use simplelog::{ColorChoice, CombinedLogger, Config, LevelFilter, TerminalMode, TermLogger, WriteLogger};
use crate::prudp::server::NexServer;

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

fn main() {
    CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Info, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
            WriteLogger::new(LevelFilter::Info, Config::default(), {
                fs::create_dir_all("log").unwrap();
                File::create(format!("log/{}.log", Local::now().to_rfc2822())).unwrap()
            })
        ]
    ).unwrap();

    dotenv::dotenv().ok();

    info!("starting auth server");

    let (auth_server, auth_server_join_handle) =
        NexServer::new(SocketAddrV4::new(*OWN_IP, *AUTH_SERVER_PORT))
        .expect("unable to startauth server");

    info!("joining auth server");

    auth_server_join_handle.join().unwrap();
}
