use std::net::UdpSocket;
use std::sync::Arc;
use crate::prudp::packet::VirtualPort;
use crate::prudp::server::Connection;

pub struct Endpoint{
    socket: Arc<UdpSocket>,
    virtual_port: VirtualPort,
}

impl Endpoint{
    pub fn get_virual_port(&self) -> VirtualPort{
        self.virtual_port
    }

    fn process_packet(connection: &Connection){
        
    }
}