use std::array;
use std::collections::HashMap;
use std::io::Write;
use std::ops::Deref;
use tokio::net::UdpSocket;
use std::sync::{Arc};
use tokio::sync::{Mutex, RwLock};
use hmac::{Hmac, Mac};
use log::{error, info, trace};
use rand::random;
use rc4::consts::U256;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use crate::prudp::packet::{flags, PacketOption, PRUDPPacket, types, VirtualPort};
use crate::prudp::packet::PacketOption::{MaximumSubstreamId, SupportedFunctions};
use crate::prudp::packet::types::SYN;
use crate::prudp::router::{Error, Router};
use crate::prudp::sockaddr::PRUDPSockAddr;


type Md5Hmac = Hmac<md5::Md5>;

/// PRUDP Socket for accepting connections to then send and recieve data from those clients
pub struct Socket(Arc<SocketImpl>, Arc<Router>, Receiver<Connection>);

#[derive(Debug)]
pub struct SocketImpl {
    virtual_port: VirtualPort,
    socket: Arc<UdpSocket>,
    access_key: &'static str,
    connections: RwLock<HashMap<PRUDPSockAddr, Arc<Mutex<Connection>>>>,
    connection_creation_sender: Sender<Connection>
}

#[derive(Debug)]
pub struct Connection {
    sock_addr: PRUDPSockAddr,
    id: u64,
    signature: [u8; 16],
}



impl Socket {
    pub async fn new(router: Arc<Router>, port: VirtualPort, access_key: &'static str) -> Result<Self, Error> {
        trace!("creating socket on router at {} on virtual port {:?}", router.get_own_address(), port);
        let (send, recv) = channel(20);

        let socket = Arc::new(
            SocketImpl::new(&router, send, port, access_key)
        );

        router.add_socket(socket.clone()).await?;

        Ok(Self(socket, router, recv))
    }

    pub async fn accept(&mut self) -> Option<Connection>{
        self.2.recv().await
    }
}

impl Drop for Socket{
    fn drop(&mut self) {
        {
            let router = self.1.clone();

            let virtual_port = self.virtual_port;
            trace!("socket dropped socket will be removed from router soon");
            // it's not that important to remove it immediately so we can delay the deletion a bit if needed
            tokio::spawn(async move {
                router.remove_socket(virtual_port).await;
                trace!("socket removed from router successfully");
            });
        }
    }
}

impl Deref for Socket{
    type Target = SocketImpl;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl SocketImpl {
    fn new(router: &Router, connection_creation_sender: Sender<Connection>, port: VirtualPort, access_key: &'static str) -> Self {
        SocketImpl {
            socket: router.get_udp_socket(),
            virtual_port: port,
            connections: Default::default(),
            access_key,
            connection_creation_sender
        }
    }

    pub fn get_virual_port(&self) -> VirtualPort {
        self.virtual_port
    }

    pub async fn process_packet(&self, connection: PRUDPSockAddr, packet: &PRUDPPacket) {
        info!("recieved packet on endpoint");

        let conn = self.connections.read().await;

        if !conn.contains_key(&connection) {
            drop(conn);

            let mut conn = self.connections.write().await;
            //only insert if we STILL dont have the connection preventing double insertion
            if !conn.contains_key(&connection) {
                conn.insert(connection, Arc::new(Mutex::new(Connection {
                    sock_addr: connection,
                    id: random(),
                    signature: [0; 16],
                })));
            }
            drop(conn);
        } else {
            drop(conn);
        }

        let connections = self.connections.read().await;

        let Some(conn) = connections.get(&connection) else {
            error!("connection is still not present after making sure connection is present, giving up.");
            return;
        };

        let conn = conn.clone();

        // dont keep holding the connections list unnescesarily
        drop(connections);

        let mut conn = conn.lock().await;

        if //((packet.header.types_and_flags.get_flags() & flags::NEED_ACK) != 0) ||
        ((packet.header.types_and_flags.get_flags() & flags::ACK) != 0) ||
            ((packet.header.types_and_flags.get_flags() & flags::RELIABLE) != 0) ||
            ((packet.header.types_and_flags.get_flags() & flags::MULTI_ACK) != 0) {
            let copy = packet.header.types_and_flags;

            unimplemented!("{:?}", copy)
        }


        match packet.header.types_and_flags.get_types() {
            types::SYN => {
                // reset heartbeat?
                let mut response_packet = packet.base_response_packet();

                response_packet.header.types_and_flags.set_types(SYN);
                response_packet.header.types_and_flags.set_flag(flags::ACK);
                response_packet.header.types_and_flags.set_flag(flags::HAS_SIZE);

                let mut hmac = Md5Hmac::new_from_slice(&[0; 16]).expect("fuck");

                let mut data = connection.regular_socket_addr.ip().octets().to_vec();
                data.extend_from_slice(&connection.regular_socket_addr.port().to_be_bytes());

                hmac.write_all(&data).expect("figuring this out was complete ass");
                let result: [u8; 16] = hmac.finalize().into_bytes()[0..16].try_into().expect("fuck");

                conn.signature = result;

                response_packet.options.push(PacketOption::ConnectionSignature(result));

                response_packet.calculate_and_assign_signature(self.access_key, None, None);

                for options in &packet.options{
                    match options{
                        SupportedFunctions(functions) => {
                            response_packet.options.push(SupportedFunctions(*functions))
                        }
                        MaximumSubstreamId(max_substream) => {
                            response_packet.options.push(MaximumSubstreamId(*max_substream))
                        },
                        _ => {/* ??? */}
                    }
                }

                let mut vec = Vec::new();

                response_packet.write_to(&mut vec).expect("somehow failed to convert backet to bytes");

                self.socket.send_to(&vec, connection.regular_socket_addr).await.expect("failed to send data back");
            }
            _ => unimplemented!("unimplemented packet type: {}", packet.header.types_and_flags.get_types())
        }
    }
}

#[cfg(test)]
mod test {
    use hmac::Mac;
    use crate::prudp::socket::Md5Hmac;

    #[test]
    fn fuck() {
        let hmac = Md5Hmac::new_from_slice(&[0; 16]).expect("fuck");
    }
}