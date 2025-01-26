use std::array;
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::io::Write;
use std::ops::Deref;
use std::pin::Pin;
use tokio::net::UdpSocket;
use std::sync::{Arc};
use tokio::sync::{Mutex, MutexGuard, RwLock};
use hmac::{Hmac, Mac};
use log::{error, info, trace, warn};
use rand::random;
use rc4::consts::{U256, U5};
use rc4::{Rc4, Rc4Core, StreamCipher};
use rc4::cipher::{KeySizeUser, StreamCipherCoreWrapper};
use rustls::internal::msgs::handshake::SessionId;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task::JoinHandle;
use tokio_stream::wrappers::ReceiverStream;
use crate::prudp::packet::{flags, PacketOption, PRUDPPacket, types, VirtualPort};
use crate::prudp::packet::flags::{ACK, HAS_SIZE, MULTI_ACK, NEED_ACK, RELIABLE};
use crate::prudp::packet::PacketOption::{ConnectionSignature, MaximumSubstreamId, SupportedFunctions};
use crate::prudp::packet::types::{CONNECT, DATA, PING, SYN};
use crate::prudp::router::{Error, Router};
use crate::prudp::sockaddr::PRUDPSockAddr;
use rc4::KeyInit;


// due to the way this is designed crashing the router thread causes deadlock, sorry ;-;
// (maybe i will fix that some day)

/// PRUDP Socket for accepting connections to then send and recieve data from those clients
pub struct Socket {
    socket_data: Arc<SocketData>,
    router: Arc<Router>,
}


type OnConnectHandlerFn = Box<dyn Fn(PRUDPPacket) -> Pin<Box<dyn Future<Output=(bool, (Box<dyn StreamCipher + Send + Sync>, Box<dyn StreamCipher + Send + Sync>))> + Send + Sync>> + Send + Sync>;
type OnDataHandlerFn = Box<dyn for<'a> Fn(PRUDPPacket, Arc<SocketData>, &'a mut MutexGuard<'_, ConnectionData>) -> Pin<Box<dyn Future<Output=()> + 'a + Send + Sync>> + Send + Sync>;

pub struct SocketData {
    virtual_port: VirtualPort,
    pub socket: Arc<UdpSocket>,
    pub access_key: &'static str,
    connections: RwLock<HashMap<PRUDPSockAddr, Arc<Mutex<ConnectionData>>>>,
    on_connect_handler: OnConnectHandlerFn,
    on_data_handler: OnDataHandlerFn,
}

pub struct ActiveConnectionData {
    pub reliable_client_counter: u16,
    pub reliable_server_counter: u16,
    pub reliable_client_queue: VecDeque<PRUDPPacket>,
    pub connection_data_channel: Sender<Vec<u8>>,
    pub server_encryption: Box<dyn StreamCipher + Send + Sync>,
    pub client_decryption: Box<dyn StreamCipher + Send + Sync>,
    pub server_session_id: u8,
}


pub struct ConnectionData {
    pub sock_addr: PRUDPSockAddr,
    pub id: u64,
    pub signature: [u8; 16],
    pub server_signature: [u8; 16],
    pub active_connection_data: Option<ActiveConnectionData>,
}


impl Socket {
    pub async fn new(
        router: Arc<Router>,
        port: VirtualPort,
        access_key: &'static str,
        on_connection_handler: OnConnectHandlerFn,
        on_data_handler: OnDataHandlerFn,
    ) -> Result<Self, Error> {
        trace!("creating socket on router at {} on virtual port {:?}", router.get_own_address(), port);

        let socket_data = Arc::new(
            SocketData::new_unbound(&router, port, access_key, on_connection_handler, on_data_handler)
        );

        router.add_socket(socket_data.clone()).await?;

        Ok(Self {
            socket_data,
            router,
        })
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        {
            let router = self.router.clone();

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

impl Deref for Socket {
    type Target = SocketData;
    fn deref(&self) -> &Self::Target {
        &self.socket_data
    }
}


impl SocketData {
    fn new_unbound(router: &Router,
                   port: VirtualPort,
                   access_key: &'static str,
                   on_connect_handler: OnConnectHandlerFn,
                   on_data_handler: OnDataHandlerFn,
    ) -> Self {
        SocketData {
            socket: router.get_udp_socket(),
            virtual_port: port,
            connections: Default::default(),
            access_key,
            on_connect_handler,
            on_data_handler,
        }
    }

    pub fn get_virual_port(&self) -> VirtualPort {
        self.virtual_port
    }

    pub async fn process_packet(self: &Arc<Self>, client_address: PRUDPSockAddr, packet: &PRUDPPacket) {
        let conn = self.connections.read().await;

        if !conn.contains_key(&client_address) {
            drop(conn);

            let mut conn = self.connections.write().await;
            //only insert if we STILL dont have the connection preventing double insertion
            if !conn.contains_key(&client_address) {
                conn.insert(client_address, Arc::new(Mutex::new(ConnectionData {
                    sock_addr: client_address,
                    id: random(),
                    signature: [0; 16],
                    server_signature: [0; 16],

                    active_connection_data: None,
                })));
            }
            drop(conn);
        } else {
            drop(conn);
        }

        let connections = self.connections.read().await;

        let Some(conn) = connections.get(&client_address) else {
            error!("connection is still not present after making sure connection is present, giving up.");
            return;
        };

        let conn = conn.clone();

        // dont keep holding the connections list unnescesarily
        drop(connections);

        let mut connection = conn.lock().await;

        if (packet.header.types_and_flags.get_flags() & ACK) != 0 {
            info!("acknowledgement recieved");
            return;
        }

        if (packet.header.types_and_flags.get_flags() & MULTI_ACK) != 0 {
            info!("acknowledgement recieved");
            return;
        }


        match packet.header.types_and_flags.get_types() {
            SYN => {
                info!("got syn");
                // reset heartbeat?
                let mut response_packet = packet.base_response_packet();

                response_packet.header.types_and_flags.set_types(SYN);
                response_packet.header.types_and_flags.set_flag(ACK);
                response_packet.header.types_and_flags.set_flag(HAS_SIZE);

                connection.signature = client_address.calculate_connection_signature();

                response_packet.options.push(ConnectionSignature(connection.signature));

                for options in &packet.options {
                    match options {
                        SupportedFunctions(functions) => {
                            response_packet.options.push(SupportedFunctions(*functions & 0x04))
                        }
                        MaximumSubstreamId(max_substream) => {
                            response_packet.options.push(MaximumSubstreamId(*max_substream))
                        }
                        _ => { /* ??? */ }
                    }
                }

                response_packet.set_sizes();

                response_packet.calculate_and_assign_signature(self.access_key, None, None);

                let mut vec = Vec::new();

                response_packet.write_to(&mut vec).expect("somehow failed to convert backet to bytes");

                self.socket.send_to(&vec, client_address.regular_socket_addr).await.expect("failed to send data back");
            }
            CONNECT => {
                info!("got connect");

                let mut response_packet = packet.base_response_packet();

                response_packet.header.types_and_flags.set_types(CONNECT);
                response_packet.header.types_and_flags.set_flag(ACK);
                response_packet.header.types_and_flags.set_flag(HAS_SIZE);

                // todo: (or not) sliding windows and stuff

                response_packet.header.session_id = packet.header.session_id;
                response_packet.header.sequence_id = 1;

                response_packet.options.push(ConnectionSignature(Default::default()));

                let mut init_seq_id = 0;

                for option in &packet.options {
                    match option {
                        MaximumSubstreamId(max_substream) => response_packet.options.push(MaximumSubstreamId(*max_substream)),
                        SupportedFunctions(funcs) => response_packet.options.push(SupportedFunctions(*funcs)),
                        ConnectionSignature(sig) => {
                            connection.server_signature = *sig
                        }
                        PacketOption::InitialSequenceId(id) => {
                            init_seq_id = *id;
                        }
                        _ => { /* ? */ }
                    }
                }

                // Splatoon doesnt use compression so we arent gonna compress unless i at some point
                // want to implement some server which requires it
                // No encryption here for the same reason

                // todo: implement something to do secure servers

                if connection.server_signature == <[u8; 16] as Default>::default() {
                    error!("didn't get connection signature from client")
                }

                response_packet.set_sizes();

                response_packet.calculate_and_assign_signature(self.access_key, None, Some(connection.server_signature));

                let mut vec = Vec::new();
                response_packet.write_to(&mut vec).expect("somehow failed to convert backet to bytes");

                self.socket.send_to(&vec, client_address.regular_socket_addr).await.expect("failed to send data back");

                let (send, recv) = channel(100);

                let (accepted, (client_decryption, server_encryption))
                    = (self.on_connect_handler)(packet.clone()).await;

                if !accepted {
                    // rejected
                    return;
                }

                connection.active_connection_data = Some(ActiveConnectionData {
                    connection_data_channel: send,
                    client_decryption,
                    server_encryption,
                    reliable_client_queue: VecDeque::new(),
                    reliable_client_counter: 2,
                    reliable_server_counter: 1,
                    server_session_id: packet.header.session_id,
                });
            }
            DATA => {
                if (packet.header.types_and_flags.get_flags() & RELIABLE) != 0 {
                    let Some(active_connection) = connection.active_connection_data.as_mut() else {
                        error!("got data packet on non active connection!");
                        return;
                    };

                    info!("ctr: {}, packet seq: {}", active_connection.reliable_client_counter, packet.header.sequence_id);

                    match active_connection.reliable_client_queue.binary_search_by_key(&packet.header.sequence_id, |p| p.header.sequence_id) {
                        Ok(_) => warn!("recieved packet twice"),
                        Err(position) => active_connection.reliable_client_queue.insert(position, packet.clone()),
                    }


                    if (packet.header.types_and_flags.get_flags() & NEED_ACK) != 0 {
                        let mut ack = packet.base_acknowledgement_packet();
                        ack.header.session_id = active_connection.server_session_id;

                        ack.set_sizes();
                        ack.calculate_and_assign_signature(self.access_key, None, Some(connection.server_signature));

                        let mut vec = Vec::new();
                        ack.write_to(&mut vec).expect("somehow failed to convert backet to bytes");

                        self.socket.send_to(&vec, client_address.regular_socket_addr).await.expect("failed to send data back");
                    }

                    while let Some(mut packet) = {
                        connection.active_connection_data.as_mut().map(|a|
                        a.reliable_client_queue
                            .front()
                            .is_some_and(|v| v.header.sequence_id == a.reliable_client_counter)
                            .then(|| a.reliable_client_queue.pop_front())).flatten().flatten()
                    } {
                        if packet.options.iter().any(|v| match v{
                            PacketOption::FragmentId(f) => (*f != 0),
                            _ => false,
                        }){
                            error!("fragmented packets are unsupported right now")
                        }

                        let active_connection = connection.active_connection_data.as_mut()
                            .expect("we litterally just recieved a packet which requires the connection to be active, failing this should be impossible");

                        active_connection.reliable_client_counter = active_connection.reliable_client_counter.overflowing_add(1).0;

                        active_connection.client_decryption.apply_keystream(&mut packet.payload);

                        // we cant divert this off to another thread we HAVE to process it now to keep order

                        (self.on_data_handler)(packet, self.clone(), &mut connection).await;
                        // ignored for now
                    }
                } else {
                    error!("unreliable packets are unimplemented");
                    unimplemented!()
                }
                //info!("{:?}", packet);
            }
            PING => {
                let ConnectionData {
                    active_connection_data,
                    server_signature,
                    ..
                } = &mut *connection;

                if (packet.header.types_and_flags.get_flags() & NEED_ACK) != 0 {
                    let Some(active_connection) = active_connection_data.as_mut() else {
                        error!("got data packet on non active connection!");
                        return;
                    };

                    let mut ack = packet.base_acknowledgement_packet();
                    ack.header.session_id = active_connection.server_session_id;

                    ack.set_sizes();
                    ack.calculate_and_assign_signature(self.access_key, None, Some(*server_signature));

                    let mut vec = Vec::new();
                    ack.write_to(&mut vec).expect("somehow failed to convert backet to bytes");

                    self.socket.send_to(&vec, client_address.regular_socket_addr).await.expect("failed to send data back");
                }
            }

            _ => unimplemented!("unimplemented packet type: {}", packet.header.types_and_flags.get_types())
        }
    }
}

#[cfg(test)]
mod test {
    use std::io::Cursor;
    use std::net::{Ipv4Addr, SocketAddrV4};
    use std::sync::Arc;
    use tokio::net::UdpSocket;
    use tokio::sync::mpsc::channel;
    use crate::prudp::packet::{PRUDPPacket, VirtualPort};
    use crate::prudp::sockaddr::PRUDPSockAddr;
    use crate::prudp::socket::SocketData;

    /*#[tokio::test]
    async fn test_connect() {
        let packet_1 = [234, 208, 1, 27, 0, 0, 175, 161, 192, 0, 0, 0, 0, 0, 36, 21, 233, 179, 203, 154, 57, 222, 219, 9, 21, 2, 29, 172, 56, 92, 0, 4, 4, 1, 0, 0, 1, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 1, 0];
        let packet_2 = [234, 208, 1, 31, 0, 0, 175, 161, 225, 0, 249, 0, 1, 0, 40, 168, 31, 138, 58, 193, 30, 134, 3, 232, 205, 245, 28, 155, 193, 198, 0, 4, 0, 0, 0, 0, 1, 16, 211, 240, 113, 188, 227, 114, 114, 30, 157, 179, 246, 55, 233, 240, 44, 197, 3, 2, 247, 244, 4, 1, 0];

        let packet_1 = PRUDPPacket::new(&mut Cursor::new(packet_1)).unwrap();
        let packet_2 = PRUDPPacket::new(&mut Cursor::new(packet_2)).unwrap();


        let (send, recv) = channel(100);

        let sock = Arc::new(SocketData {
            connections: Default::default(),
            access_key: "6f599f81",
            virtual_port: VirtualPort(0),
            socket: Arc::new(UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 10000)).await.unwrap()),
            connection_creation_sender: send,
        });
        println!("sent: {:?}", packet_1);
        sock.process_packet(PRUDPSockAddr {
            virtual_port: VirtualPort(0),
            regular_socket_addr: SocketAddrV4::new(Ipv4Addr::LOCALHOST, 2469),
        }, &packet_1).await;
        println!("sent: {:?}", packet_2);
        sock.process_packet(PRUDPSockAddr {
            virtual_port: VirtualPort(0),
            regular_socket_addr: SocketAddrV4::new(Ipv4Addr::LOCALHOST, 2469),
        }, &packet_2).await;
    }*/
}