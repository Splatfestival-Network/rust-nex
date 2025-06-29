use std::env;
use std::net::{Ipv4Addr, SocketAddrV4};
use macros::{method_id, rmc_proto, RmcSerialize};
use once_cell::sync::Lazy;
use tonic::transport::Server;
use rust_nex::define_rmc_proto;
use rust_nex::prudp::station_url::StationUrl;
use crate::nex::account::Account;
use crate::rmc::response::ErrorCode;

pub static OWN_IP_PRIVATE: Lazy<Ipv4Addr> = Lazy::new(|| {
    env::var("SERVER_IP")
        .ok()
        .and_then(|s| s.parse().ok())
        .expect("no private ip specified")
});

pub static OWN_IP_PUBLIC: Lazy<Ipv4Addr> = Lazy::new(|| {
    env::var("SERVER_IP_PUBLIC")
        .ok()
        .and_then(|s| s.parse().ok())
        .expect("no private ip specified")
});

pub static SERVER_PORT: Lazy<u16> = Lazy::new(|| {
    env::var("SERVER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10000)
});

pub static KERBEROS_SERVER_PASSWORD: Lazy<String> = Lazy::new(|| {
    env::var("AUTH_SERVER_PASSWORD")
        .ok()
        .unwrap_or("password".to_owned())
});

pub static AUTH_SERVER_ACCOUNT: Lazy<Account> =
    Lazy::new(|| Account::new(1, "Quazal Authentication", &KERBEROS_SERVER_PASSWORD));
pub static SECURE_SERVER_ACCOUNT: Lazy<Account> =
    Lazy::new(|| Account::new(2, "Quazal Rendez-Vous", &KERBEROS_SERVER_PASSWORD));


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

