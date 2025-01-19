use std::net::SocketAddrV4;
use crate::prudp::packet::VirtualPort;

pub struct PRUDPSockAddr{
    pub regular_socket_addr: SocketAddrV4,
    pub virtual_port: VirtualPort
}