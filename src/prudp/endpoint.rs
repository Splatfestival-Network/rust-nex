use std::collections::HashMap;
use std::net::UdpSocket;
use std::sync::{Arc, RwLock};
use log::{error, info};
use rand::random;
use crate::prudp::packet::{flags, PRUDPPacket, VirtualPort};
use crate::prudp::sockaddr::PRUDPSockAddr;

#[derive(Debug)]
pub struct Endpoint{
    virtual_port: VirtualPort,
    socket: Arc<UdpSocket>,
    connections: RwLock<HashMap<PRUDPSockAddr, Connection>>
}

#[derive(Debug)]
pub struct Connection{
    sock_addr: PRUDPSockAddr,
    id: u64
}

impl Endpoint{
    pub fn new(socket: Arc<UdpSocket>, port: VirtualPort) ->  Self{
        Self{
            socket,
            virtual_port: port,
            connections: Default::default()
        }
    }

    pub fn get_virual_port(&self) -> VirtualPort{
        self.virtual_port
    }

    pub fn process_packet(&self, connection: PRUDPSockAddr, packet: &PRUDPPacket){
        info!("recieved packet on endpoint");

        let conn = self.connections.read().expect("poison");

        if !conn.contains_key(&connection){
            drop(conn);

            let mut conn = self.connections.write().expect("poison");
            //only insert if we STILL dont have the connection preventing double insertion
            if !conn.contains_key(&connection) {
                conn.insert(connection, Connection {
                    sock_addr: connection,
                    id: random()
                });
            }
            drop(conn);
        } else {
            drop(conn);
        }

        let conn = self.connections.read().expect("poison");

        let Some(conn) = conn.get(&connection) else {
            error!("connection is still not present after making sure connection is present, giving up.");
            return;
        };

        if ((packet.header.types_and_flags.get_flags() & flags::NEED_ACK) != 0) ||
           ((packet.header.types_and_flags.get_flags() & flags::ACK) != 0) ||
           ((packet.header.types_and_flags.get_flags() & flags::RELIABLE) != 0) ||
            ((packet.header.types_and_flags.get_flags() & flags::MULTI_ACK) != 0) {
            unimplemented!("{:?}", packet.header.types_and_flags)
        }





    }
}