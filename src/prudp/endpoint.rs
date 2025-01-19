use std::net::UdpSocket;
use std::sync::Arc;
use log::info;
use crate::prudp::packet::{PRUDPPacket, VirtualPort};
use crate::prudp::server::Connection;

#[derive(Debug)]
pub struct Endpoint{
    virtual_port: VirtualPort,
}

impl Endpoint{
    pub fn new(port: VirtualPort) ->  Self{
        Self{
            virtual_port: port
        }
    }

    pub fn get_virual_port(&self) -> VirtualPort{
        self.virtual_port
    }

    pub fn process_packet(&self, connection: &Connection, packet: &PRUDPPacket){
        info!("recieved packet on endpoint")

    }
}