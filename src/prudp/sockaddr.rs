use std::io::Write;
use std::net::SocketAddrV4;
use hmac::{Hmac, Mac};
use crate::prudp::packet::VirtualPort;

type Md5Hmac = Hmac<md5::Md5>;

#[derive(Eq, PartialEq, Hash, Debug, Copy, Clone)]
pub struct PRUDPSockAddr{
    pub regular_socket_addr: SocketAddrV4,
    pub virtual_port: VirtualPort
}

impl PRUDPSockAddr{
    pub fn calculate_connection_signature(&self) -> [u8; 16] {
        let mut hmac = Md5Hmac::new_from_slice(&[0; 16]).expect("fuck");

        let mut data = self.regular_socket_addr.ip().octets().to_vec();
        data.extend_from_slice(&self.regular_socket_addr.port().to_be_bytes());

        hmac.write_all(&data).expect("figuring this out was complete ass");
        let result: [u8; 16] = hmac.finalize().into_bytes()[0..16].try_into().expect("fuck");
        result
    }
}