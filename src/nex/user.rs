use std::net::{Ipv4Addr, SocketAddrV4};
use macros::rmc_struct;
use crate::define_rmc_proto;
use crate::prudp::station_url::{nat_types, StationUrl};
use crate::prudp::station_url::Type::PRUDPS;
use crate::prudp::station_url::UrlOptions::{Address, NatFiltering, NatMapping, NatType, Port, PrincipalID, RVConnectionID};
use crate::rmc::protocols::secure::{RemoteAuth, RawAuthInfo, RawAuth, Auth};
use crate::rmc::response::ErrorCode;
use crate::rmc::structures::qresult::QResult;

define_rmc_proto!(
    proto UserProtocol{
        Auth
    }
);

#[rmc_struct(UserProtocol)]
pub struct User {
    pub pid: u32,
    pub ip: SocketAddrV4,
}

impl Auth for User{
    async fn register(&self, station_urls: Vec<String>) -> Result<(QResult, u32, String), ErrorCode> {
        let public_station = StationUrl{
            url_type: PRUDPS,
            options: vec![
                RVConnectionID(0),
                Address(*self.ip.ip()),
                Port(self.ip.port()),
                NatFiltering(0),
                NatMapping(0),
                NatType(nat_types::BEHIND_NAT),
                PrincipalID(self.pid),
            ]
        };

        let result = QResult::success(ErrorCode::Core_Unknown);

        Ok((result, 0, public_station.to_string()))
    }
}



