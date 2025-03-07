use crate::prudp::packet::flags::{ACK, HAS_SIZE, MULTI_ACK, NEED_ACK, RELIABLE};
use crate::prudp::packet::types::{CONNECT, DATA, DISCONNECT, PING, SYN};
use crate::prudp::packet::PacketOption::{ConnectionSignature, FragmentId, MaximumSubstreamId, SupportedFunctions};
use crate::prudp::packet::{PRUDPHeader, PRUDPPacket, PacketOption, TypesFlags, VirtualPort};
use crate::prudp::router::{Error, Router};
use crate::prudp::sockaddr::PRUDPSockAddr;
use crate::web::DirectionalData::Outgoing;
use crate::web::WEB_DATA;
use async_trait::async_trait;
use hmac::digest::consts::U5;
use log::info;
use log::{error, trace, warn};
use once_cell::sync::Lazy;
use rand::random;
use rc4::{Key, KeyInit, Rc4, StreamCipher};
use rocket::http::hyper::body::HttpBody;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::marker::PhantomData;
use std::mem;
use std::net::SocketAddrV4;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::{Arc, Weak};
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::{Mutex, RwLock};
use tokio_stream::Stream;
// due to the way this is designed crashing the router thread causes deadlock, sorry ;-;
// (maybe i will fix that some day)

/// PRUDP Socket for accepting connections to then send and recieve data from those clients


pub struct EncryptionPair<T: StreamCipher + Send> {
    pub send: T,
    pub recv: T,
}

impl<T: StreamCipher + Send> EncryptionPair<T> {
    fn init_both<F: Fn() -> T>(func: F) -> Self {
        Self {
            recv: func(),
            send: func(),
        }
    }
}
/*
    pub async fn process_packet(
        self: &Arc<Self>,
        client_address: PRUDPSockAddr,
        packet: &PRUDPPacket,
    ) {
        let conn = self.connections.read().await;

        if !conn.contains_key(&client_address) {
            drop(conn);

            let mut conn = self.connections.write().await;
            //only insert if we STILL dont have the connection preventing double insertion
            if !conn.contains_key(&client_address) {
                conn.insert(
                    client_address,
                    (
                        Arc::new(Mutex::new(ConnectionData {
                            sock_addr: client_address,
                            id: random(),
                            signature: [0; 16],
                            server_signature: [0; 16],

                            active_connection_data: None,
                        })),
                        Arc::new(Mutex::new(())),
                    ),
                );
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

        let mutual_exclusion_packet_handeling_mtx = conn.1.lock().await;
        let mut connection = conn.0.lock().await;

        if (packet.header.types_and_flags.get_flags() & ACK) != 0 {
            //todo: handle acknowledgements and resending packets propperly
            println!("got ack");
            return;
        }

        if (packet.header.types_and_flags.get_flags() & MULTI_ACK) != 0 {
            println!("got multi ack");
            return;
        }

        match packet.header.types_and_flags.get_types() {
            SYN => {
                println!("got syn");
                // reset heartbeat?
                let mut response_packet = packet.base_response_packet();

                response_packet.header.types_and_flags.set_types(SYN);
                response_packet.header.types_and_flags.set_flag(ACK);
                response_packet.header.types_and_flags.set_flag(HAS_SIZE);

                connection.signature = client_address.calculate_connection_signature();

                response_packet
                    .options
                    .push(ConnectionSignature(connection.signature));

                for options in &packet.options {
                    match options {
                        SupportedFunctions(functions) => response_packet
                            .options
                            .push(SupportedFunctions(*functions & 0x04)),
                        MaximumSubstreamId(max_substream) => response_packet
                            .options
                            .push(MaximumSubstreamId(*max_substream)),
                        _ => { /* ??? */ }
                    }
                }

                response_packet.set_sizes();

                response_packet.calculate_and_assign_signature(self.access_key, None, None);

                let mut vec = Vec::new();

                response_packet
                    .write_to(&mut vec)
                    .expect("somehow failed to convert backet to bytes");

                self.socket
                    .send_to(&vec, client_address.regular_socket_addr)
                    .await
                    .expect("failed to send data back");
            }
            CONNECT => {
                println!("got connect");
                let Some(MaximumSubstreamId(max_substream)) = packet
                    .options
                    .iter()
                    .find(|v| matches!(v, MaximumSubstreamId(_)))
                else {
                    return;
                };

                let Some((response_data, encryption_pairs, active_secure_connection_data)) =
                    (self.on_connect_handler)(packet.clone(), *max_substream).await
                else {
                    error!("invalid connection request");
                    return;
                };

                connection.active_connection_data = Some(ActiveConnectionData {
                    encryption_pairs,
                    reliable_client_queue: VecDeque::new(),
                    reliable_client_counter: 2,
                    reliable_server_counter: 1,
                    server_session_id: packet.header.session_id,
                    active_secure_connection_data,
                    connection_id: random(),
                });

                let mut response_packet = packet.base_response_packet();

                response_packet.payload = response_data;

                response_packet.header.types_and_flags.set_types(CONNECT);
                response_packet.header.types_and_flags.set_flag(ACK);
                response_packet.header.types_and_flags.set_flag(HAS_SIZE);

                // todo: (or not) sliding windows and stuff

                response_packet.header.session_id = packet.header.session_id;
                response_packet.header.sequence_id = 1;

                response_packet
                    .options
                    .push(ConnectionSignature(Default::default()));

                //let mut init_seq_id = 0;

                for option in &packet.options {
                    match option {
                        MaximumSubstreamId(max_substream) => response_packet
                            .options
                            .push(MaximumSubstreamId(*max_substream)),
                        SupportedFunctions(funcs) => {
                            response_packet.options.push(SupportedFunctions(*funcs))
                        }
                        ConnectionSignature(sig) => connection.server_signature = *sig,
                        PacketOption::InitialSequenceId(_id) => {
                            //init_seq_id = *id;
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

                response_packet.calculate_and_assign_signature(
                    self.access_key,
                    None,
                    Some(connection.server_signature),
                );

                let mut vec = Vec::new();
                response_packet
                    .write_to(&mut vec)
                    .expect("somehow failed to convert backet to bytes");

                self.socket
                    .send_to(&vec, client_address.regular_socket_addr)
                    .await
                    .expect("failed to send data back");
            }
            DATA => {
                if (packet.header.types_and_flags.get_flags() & RELIABLE) != 0 {
                    let Some(active_connection) = connection.active_connection_data.as_mut() else {
                        error!("got data packet on non active connection!");
                        return;
                    };

                    match active_connection
                        .reliable_client_queue
                        .binary_search_by_key(&packet.header.sequence_id, |p| p.header.sequence_id)
                    {
                        Ok(_) => warn!("recieved packet twice"),
                        Err(position) => active_connection
                            .reliable_client_queue
                            .insert(position, packet.clone()),
                    }

                    if (packet.header.types_and_flags.get_flags() & NEED_ACK) != 0 {
                        let mut ack = packet.base_acknowledgement_packet();
                        ack.header.session_id = active_connection.server_session_id;

                        ack.set_sizes();
                        let potential_session_key = connection
                            .active_connection_data
                            .as_ref()
                            .unwrap()
                            .active_secure_connection_data
                            .as_ref()
                            .map(|s| s.session_key);

                        ack.calculate_and_assign_signature(
                            self.access_key,
                            potential_session_key,
                            Some(connection.server_signature),
                        );

                        let mut vec = Vec::new();
                        ack.write_to(&mut vec)
                            .expect("somehow failed to convert backet to bytes");

                        self.socket
                            .send_to(&vec, client_address.regular_socket_addr)
                            .await
                            .expect("failed to send data back");
                    }
                    drop(connection);
                    while let Some(mut packet) = {
                        let mut locked = conn.0.lock().await;

                        let packet = locked
                            .active_connection_data
                            .as_mut()
                            .map(|a| {
                                a.reliable_client_queue
                                    .front()
                                    .is_some_and(|v| {
                                        v.header.sequence_id == a.reliable_client_counter
                                    })
                                    .then(|| a.reliable_client_queue.pop_front())
                            })
                            .flatten()
                            .flatten();

                        drop(locked);
                        packet
                    } {
                        if packet.options.iter().any(|v| match v {
                            PacketOption::FragmentId(f) => *f != 0,
                            _ => false,
                        }) {
                            error!("fragmented packets are unsupported right now")
                        }

                        let mut locked = conn.0.lock().await;

                        let active_connection = locked.active_connection_data.as_mut()
                            .expect("we litterally just recieved a packet which requires the connection to be active, failing this should be impossible");

                        active_connection.reliable_client_counter = active_connection
                            .reliable_client_counter
                            .overflowing_add(1)
                            .0;

                        let Some(stream) = active_connection
                            .encryption_pairs
                            .get_mut(packet.header.substream_id as usize)
                            .map(|e| &mut e.recv)
                        else {
                            return;
                        };

                        stream.apply_keystream(&mut packet.payload);

                        drop(locked);
                        // we cant divert this off to another thread we HAVE to process it now to keep order

                        (self.on_data_handler)(packet, self.clone(), conn.0.clone()).await;
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

                info!("got ping");

                if (packet.header.types_and_flags.get_flags() & NEED_ACK) != 0 {
                    let Some(active_connection) = active_connection_data.as_mut() else {
                        error!("got data packet on non active connection!");
                        return;
                    };

                    let mut ack = packet.base_acknowledgement_packet();
                    ack.header.session_id = active_connection.server_session_id;

                    ack.set_sizes();

                    let potential_session_key = active_connection
                        .active_secure_connection_data
                        .as_ref()
                        .map(|s| s.session_key);

                    ack.calculate_and_assign_signature(
                        self.access_key,
                        potential_session_key,
                        Some(*server_signature),
                    );

                    let mut vec = Vec::new();
                    ack.write_to(&mut vec)
                        .expect("somehow failed to convert backet to bytes");

                    self.socket
                        .send_to(&vec, client_address.regular_socket_addr)
                        .await
                        .expect("failed to send data back");
                }
            }
            DISCONNECT => {
                println!("got disconnect");
                let Some(active_connection) = &connection.active_connection_data else {
                    return;
                };

                let mut ack = packet.base_acknowledgement_packet();

                ack.header.session_id = active_connection.server_session_id;

                ack.set_sizes();

                let potential_session_key = active_connection
                    .active_secure_connection_data
                    .as_ref()
                    .map(|s| s.session_key);

                ack.calculate_and_assign_signature(
                    self.access_key,
                    potential_session_key,
                    Some(connection.server_signature),
                );

                let mut vec = Vec::new();
                ack.write_to(&mut vec)
                    .expect("somehow failed to convert backet to bytes");

                self.socket
                    .send_to(&vec, client_address.regular_socket_addr)
                    .await
                    .expect("failed to send data back");
                self.socket
                    .send_to(&vec, client_address.regular_socket_addr)
                    .await
                    .expect("failed to send data back");
                self.socket
                    .send_to(&vec, client_address.regular_socket_addr)
                    .await
                    .expect("failed to send data back");
            }
            _ => error!(
                "unimplemented packet type: {}",
                packet.header.types_and_flags.get_types()
            ),
        }

        drop(mutual_exclusion_packet_handeling_mtx)
    }*/
/*
impl ConnectionData {
    pub async fn finish_and_send_packet_to(
        &mut self,
        socket: &SocketData,
        mut packet: PRUDPPacket,
    ) {
        let mut web = WEB_DATA.lock().await;
        web.data.push((
            self.sock_addr.regular_socket_addr,
            Outgoing(hex::encode(&packet.payload)),
        ));
        drop(web);

        if (packet.header.types_and_flags.get_flags() & RELIABLE) != 0 {
            let Some(active_connection) = self.active_connection_data.as_mut() else {
                error!("tried to send a secure packet to an inactive connection");
                return;
            };

            packet.header.sequence_id = active_connection.reliable_server_counter;
            active_connection.reliable_server_counter += 1;

            let Some(encryption) = active_connection
                .encryption_pairs
                .get_mut(packet.header.substream_id as usize)
                .map(|e| &mut e.send)
            else {
                return;
            };

            encryption.apply_keystream(&mut packet.payload);
        }

        packet.header.session_id = self
            .active_connection_data
            .as_ref()
            .map(|v| v.server_session_id)
            .unwrap_or_default();

        packet.header.source_port = socket.virtual_port;
        packet.header.destination_port = self.sock_addr.virtual_port;

        packet.set_sizes();

        let potential_session_key = self
            .active_connection_data
            .as_ref()
            .unwrap()
            .active_secure_connection_data
            .as_ref()
            .map(|s| s.session_key);

        packet.calculate_and_assign_signature(
            socket.access_key,
            potential_session_key,
            Some(self.server_signature),
        );
        let mut vec = Vec::new();

        packet
            .write_to(&mut vec)
            .expect("somehow failed to convert backet to bytes");

        if let Err(e) = socket
            .socket
            .send_to(&vec, self.sock_addr.regular_socket_addr)
            .await
        {
            error!("unable to send packet to destination: {}", e);
        }
    }
}*/

pub struct NewEncryptionPair<E: StreamCipher> {
    pub send: E,
    pub recv: E,
}

pub struct CommonConnection {
    pub user_id: u32,
    pub socket_addr: PRUDPSockAddr,
    pub server_port: VirtualPort,
    session_id: u8,
}

struct InternalConnection<E: CryptoHandlerConnectionInstance> {
    common: Arc<CommonConnection>,
    reliable_server_counter: u16,
    reliable_client_counter: u16,
    // maybe add connection id(need to see if its even needed)
    crypto_handler_instance: E,
    data_sender: Sender<Vec<u8>>,
    socket: Arc<UdpSocket>
}

impl<E: CryptoHandlerConnectionInstance> Deref for InternalConnection<E>{
    type Target = CommonConnection;
    fn deref(&self) -> &Self::Target {
        &self.common
    }
}

impl<E: CryptoHandlerConnectionInstance> InternalConnection<E>{
    fn next_server_count(&mut self) -> u16{
        let (val, _) = self.reliable_server_counter.overflowing_add(1);
        self.reliable_server_counter = val;
        val
    }
}

pub struct ExternalConnection {
    sending: SendingConnection,
    data_receiver: Receiver<Vec<u8>>,
}

#[derive(Clone)]
pub struct SendingConnection{
    common: Arc<CommonConnection>,
    inernal: Weak<Mutex<dyn AnyInternalConnection>>
}

pub struct CommonSocket {
    pub virtual_port: VirtualPort,
    _phantom_unconstructible: PhantomData<()>,
}

pub(super) struct InternalSocket<T: CryptoHandler> {
    common: Arc<CommonSocket>,
    socket: Arc<UdpSocket>,
    crypto_handler: T,
    // perf note: change the code to use RwLock here instead to avoid connections being able to block one another before the data is sent off.
    internal_connections: Arc<
        Mutex<BTreeMap<PRUDPSockAddr, Arc<Mutex<InternalConnection<T::CryptoConnectionInstance>>>>>,
    >,
    connection_establishment_data_sender: Mutex<Option<Sender<PRUDPPacket>>>,
    connection_sender: Sender<ExternalConnection>,
}

pub struct ExternalSocket {
    common: Arc<CommonSocket>,
    connection_receiver: Receiver<ExternalConnection>,
    internal: Weak<dyn AnyInternalSocket>,
}

impl ExternalSocket{
    pub async fn connect(&mut self, addr: PRUDPSockAddr) -> Option<ExternalConnection>{
        let socket = self.internal.upgrade()?;

        socket.connect(addr).await;

        self.connection_receiver.recv().await
    }

    pub async fn accept(&mut self) -> Option<ExternalConnection>{
        self.connection_receiver.recv().await
    }
}

impl Deref for ExternalSocket {
    type Target = CommonSocket;
    fn deref(&self) -> &Self::Target {
        &self.common
    }
}

impl<T: CryptoHandler> Deref for InternalSocket<T> {
    type Target = CommonSocket;
    fn deref(&self) -> &Self::Target {
        &self.common
    }
}

#[async_trait]
pub(super) trait AnyInternalSocket:
    Send + Sync + Deref<Target = CommonSocket> + 'static
{
    async fn recieve_packet(&self, address: PRUDPSockAddr, packet: PRUDPPacket);
    async fn connect(&self, address: PRUDPSockAddr) -> Option<()>;
}

#[async_trait]
pub(super) trait AnyInternalConnection:
    Send + Sync + Deref<Target = CommonConnection> + 'static
{
    async fn send_data_packet(&mut self, data: Vec<u8>);
}

#[async_trait]
impl<T: CryptoHandlerConnectionInstance> AnyInternalConnection for InternalConnection<T>{
    async fn send_data_packet(&mut self, data: Vec<u8>) {
        let mut packet = PRUDPPacket{
            header: PRUDPHeader{
                sequence_id: self.next_server_count(),
                substream_id: 0,
                session_id: self.session_id,
                types_and_flags: TypesFlags::default().types(DATA).flags(RELIABLE | NEED_ACK),
                destination_port: self.common.socket_addr.virtual_port,
                source_port: self.server_port,
                ..Default::default()
            },
            payload: data,
            options: vec![FragmentId(0)],
            ..Default::default()
        };

        self.crypto_handler_instance.encrypt_outgoing(0, &mut packet.payload[..]);

        packet.set_sizes();

        self.crypto_handler_instance.sign_packet(&mut packet);

        packet.set_sizes();

        let mut vec = Vec::new();

        packet
            .write_to(&mut vec)
            .expect("somehow failed to convert backet to bytes");

        self.socket
            .send_to(&vec, self.socket_addr.regular_socket_addr)
            .await
            .expect("failed to send data back");
    }
}

impl<T: CryptoHandler> InternalSocket<T> {
    async fn send_packet_unbuffered(&self, dest: PRUDPSockAddr, mut packet: PRUDPPacket) {
        packet.set_sizes();

        let mut vec = Vec::new();

        packet
            .write_to(&mut vec)
            .expect("somehow failed to convert backet to bytes");

        println!("{}", hex::encode(&vec));

        self.socket
            .send_to(&vec, dest.regular_socket_addr)
            .await
            .expect("failed to send data back");
    }

    async fn handle_syn(&self, address: PRUDPSockAddr, packet: PRUDPPacket) {
        info!("got syn");

        let mut response = packet.base_acknowledgement_packet();

        let signature = address.calculate_connection_signature();

        response.options.push(ConnectionSignature(signature));

        // todo: refactor this to be more readable(low priority cause it doesnt change anything api wise)
        for options in &packet.options {
            match options {
                SupportedFunctions(functions) => response
                    .options
                    .push(SupportedFunctions(*functions & 0x04)),
                MaximumSubstreamId(max_substream) => response
                    .options
                    .push(MaximumSubstreamId(*max_substream)),
                _ => { /* ??? */ }
            }
        }

        response.header.types_and_flags.set_flag(HAS_SIZE | ACK);

        self.crypto_handler.sign_pre_handshake(&mut response);

        self.send_packet_unbuffered(address, response)
            .await;
    }

    async fn connection_thread(
        socket: Arc<UdpSocket>,
        self_port: VirtualPort,
        connection: Arc<Mutex<InternalConnection<T::CryptoConnectionInstance>>>,
        mut data_recv: Receiver<Vec<u8>>
    ) {
        //todo: handle stuff like resending packets if they arent acknowledged in here
        while let Some(data) = data_recv.recv().await{
            let mut locked_conn = connection.lock().await;
            let packet = PRUDPPacket{
                header: PRUDPHeader{
                    sequence_id: locked_conn.next_server_count(),
                    substream_id: 0,
                    session_id: locked_conn.session_id,
                    types_and_flags: TypesFlags::default().types(DATA).flags(RELIABLE | NEED_ACK),
                    destination_port: locked_conn.common.socket_addr.virtual_port,
                    source_port: self_port,
                    ..Default::default()
                },
                payload: data,
                options: vec![FragmentId(0)],
                ..Default::default()
            };

            //packet.






        }
    }

    async fn create_connection(
        &self,
        crypto_handler_instance: T::CryptoConnectionInstance,
        socket_addr: PRUDPSockAddr,
        session_id: u8,
    ) {
        let common = Arc::new(CommonConnection {
            user_id: crypto_handler_instance.get_user_id(),
            socket_addr,
            session_id,
            server_port: self.virtual_port
        });

        let (data_sender_from_client, data_receiver_from_client) = channel(16);

        let internal = InternalConnection {
            common: common.clone(),
            crypto_handler_instance,
            reliable_client_counter: 2,
            reliable_server_counter: 1,
            data_sender: data_sender_from_client,
            socket: self.socket.clone()
        };

        let internal = Arc::new(Mutex::new(internal));

        let dyn_internal: Arc<Mutex<dyn AnyInternalConnection>> = internal.clone();

        let external = ExternalConnection {
            sending: SendingConnection{
                common,
                inernal: Arc::downgrade(&dyn_internal)
            },
            data_receiver: data_receiver_from_client,

        };





        let mut connections = self.internal_connections.lock().await;

        connections.insert(socket_addr, internal.clone());

        drop(connections);

        self.connection_sender
            .send(external)
            .await
            .expect("connection to external socket lost");
    }

    async fn handle_connect(&self, address: PRUDPSockAddr, packet: PRUDPPacket) {
        info!("got connect");
        let Some(MaximumSubstreamId(max_substream)) = packet
            .options
            .iter()
            .find(|v| matches!(v, MaximumSubstreamId(_)))
        else {
            return;
        };

        let remote_signature = address.calculate_connection_signature();

        let Some(ConnectionSignature(own_signature)) = packet
            .options
            .iter()
            .find(|p| matches!(p, ConnectionSignature(_)))
        else {
            error!("didnt get connection signature from client");
            return;
        };

        let session_id = packet.header.session_id;

        let (return_data, crypto) = self.crypto_handler.instantiate(
            remote_signature,
            *own_signature,
            &packet.payload,
            *max_substream,
        );

        let mut response = packet.base_acknowledgement_packet();
        response.header.types_and_flags.set_flag(HAS_SIZE | ACK);
        response.header.session_id = session_id;
        response.payload = return_data;

        crypto.sign_connect(&mut response);

        self.create_connection(crypto, address, session_id).await;

        self.send_packet_unbuffered(address, response).await;
    }

    async fn handle_data(&self, address: PRUDPSockAddr, mut packet: PRUDPPacket) {
        info!("got data");

        if packet.header.types_and_flags.get_flags() & (NEED_ACK | RELIABLE) !=  (NEED_ACK | RELIABLE){
            error!("invalid or unimplemented packet flags");
        }

        let connections = self.internal_connections.lock().await;
        let Some(conn) = connections.get(&address) else{
            error!("tried to send data on inactive connection!");
            return
        };
        let conn = conn.clone();
        drop(connections);

        let mut conn = conn.lock().await;

        conn.crypto_handler_instance.decrypt_incoming(packet.header.substream_id, &mut packet.payload[..]);

        let mut data = Vec::new();

        mem::swap(&mut data, &mut packet.payload);

        conn.data_sender.send(data).await.expect("socket died");

        if packet.header.types_and_flags.get_flags() & NEED_ACK == 0{
            return;
        }

        let mut response = packet.base_acknowledgement_packet();
        response.header.types_and_flags.set_flag(HAS_SIZE | ACK);
        response.header.session_id = conn.session_id;

        conn.crypto_handler_instance.sign_packet(&mut response);

        self.send_packet_unbuffered(address, response).await;
    }

    async fn handle_ping(&self, address: PRUDPSockAddr, packet: PRUDPPacket){
        let connections = self.internal_connections.lock().await;
        let Some(conn) = connections.get(&address) else{
            error!("tried to send data on inactive connection!");
            return
        };
        let conn = conn.clone();
        drop(connections);

        let mut conn = conn.lock().await;

        let mut response = packet.base_acknowledgement_packet();
        response.header.types_and_flags.set_flag(HAS_SIZE | ACK);
        response.header.session_id = conn.session_id;

        conn.crypto_handler_instance.sign_packet(&mut response);

        self.send_packet_unbuffered(address, response).await;
    }

    async fn handle_disconnect(&self, address: PRUDPSockAddr, packet: PRUDPPacket){
        let connections = self.internal_connections.lock().await;
        let Some(conn) = connections.get(&address) else{
            error!("tried to send data on inactive connection!");
            return
        };
        let conn = conn.clone();
        drop(connections);

        let mut conn = conn.lock().await;

        let mut response = packet.base_acknowledgement_packet();
        response.header.types_and_flags.set_flag(HAS_SIZE | ACK);
        response.header.session_id = conn.session_id;

        conn.crypto_handler_instance.sign_packet(&mut response);

        self.send_packet_unbuffered(address, response.clone()).await;
        self.send_packet_unbuffered(address, response.clone()).await;
        self.send_packet_unbuffered(address, response).await;
    }
}

#[async_trait]
impl<T: CryptoHandler> AnyInternalSocket for InternalSocket<T> {
    async fn recieve_packet(&self, address: PRUDPSockAddr, packet: PRUDPPacket) {
        // todo: handle acks
        if (packet.header.types_and_flags.get_flags() & ACK) != 0 {
            info!("got ack");
            if packet.header.types_and_flags.get_types() == SYN ||
                packet.header.types_and_flags.get_types() == CONNECT{
                let sender = self.connection_establishment_data_sender.lock().await;
                info!("redirecting ack to active connection establishment code");

                if let Some(conn) = sender.as_ref(){
                    if let Err(e) = conn.send(packet).await {
                        error!("error whilest sending data to connection establishment: {}", e);
                    }
                } else {
                    error!("got connection response without the active reciever being present");
                }
            }
            return;
        }

        if (packet.header.types_and_flags.get_flags() & MULTI_ACK) != 0 {
            info!("got multi ack");
            return;
        }

        match packet.header.types_and_flags.get_types() {
            SYN => self.handle_syn(address, packet).await,
            CONNECT => self.handle_connect(address, packet).await,
            DATA => self.handle_data(address, packet).await,
            _ => {
                error!(
                    "unimplemented packet type: {}",
                    packet.header.types_and_flags.get_types()
                )
            }
        }
    }

    async fn connect(&self, address: PRUDPSockAddr) -> Option<()> {
        let (send, mut recv) = channel(10);

        let mut sender = self.connection_establishment_data_sender.lock().await;
        *sender = Some(send);
        drop(sender);

        let remote_signature = address.calculate_connection_signature();

        let packet = PRUDPPacket{
            header: PRUDPHeader{
                source_port: self.virtual_port,
                destination_port: address.virtual_port,
                types_and_flags: TypesFlags::default().types(SYN).flags(NEED_ACK),
                ..Default::default()
            },
            options: vec![
                SupportedFunctions(0x104),
                MaximumSubstreamId(1),
                ConnectionSignature(remote_signature)
            ],
            ..Default::default()
        };



        self.send_packet_unbuffered(address, packet).await;

        let Some(syn_ack_packet) = recv.recv().await else{
            error!("what");
            return None;
        };

        let Some(ConnectionSignature(own_signature)) = syn_ack_packet
            .options
            .iter()
            .find(|p| matches!(p, ConnectionSignature(_)))
        else {
            error!("didnt get connection signature from remote partner");
            return None;
        };



        let packet = PRUDPPacket{
            header: PRUDPHeader{
                source_port: self.virtual_port,
                destination_port: address.virtual_port,
                types_and_flags: TypesFlags::default().types(CONNECT).flags(NEED_ACK),
                ..Default::default()
            },
            options: vec![
                SupportedFunctions(0x04),
                MaximumSubstreamId(1),
                ConnectionSignature(remote_signature)
            ],
            ..Default::default()
        };

        self.send_packet_unbuffered(address, packet).await;

        let Some(connect_ack_packet) = recv.recv().await else{
            error!("what");
            return None;
        };

        let (_, crypt) = self.crypto_handler.instantiate(remote_signature, *own_signature, &[], 1);

        //todo: make this work for secure servers as well
        self.create_connection(crypt, address, 0).await;

        Some(())
    }
}

pub(super) fn new_socket_pair<T: CryptoHandler>(
    virtual_port: VirtualPort,
    encryption: T,
    socket: Arc<UdpSocket>,
) -> (Arc<InternalSocket<T>>, ExternalSocket) {
    let common = Arc::new(CommonSocket {
        virtual_port,
        _phantom_unconstructible: Default::default(),
    });

    let (connection_send, connection_recv) = channel(16);

    let internal = Arc::new(InternalSocket {
        common: common.clone(),
        connection_sender: connection_send,
        crypto_handler: encryption,
        internal_connections: Default::default(),
        connection_establishment_data_sender: Default::default(),
        socket,
    });

    let dyn_internal: Arc<dyn AnyInternalSocket> = internal.clone();

    let external = ExternalSocket {
        common,
        connection_receiver: connection_recv,
        internal: Arc::downgrade(&dyn_internal),
    };

    (internal, external)
}

pub trait CryptoHandlerConnectionInstance: Send + Sync + 'static {
    type Encryption: StreamCipher + Send;

    fn decrypt_incoming(&mut self, substream: u8, data: &mut [u8]);
    fn encrypt_outgoing(&mut self, substream: u8, data: &mut [u8]);

    fn get_user_id(&self) -> u32;
    fn sign_connect(&self, packet: &mut PRUDPPacket);
    fn sign_packet(&self, packet: &mut PRUDPPacket);
    fn verify_packet(&self, packet: &PRUDPPacket) -> bool;
}

pub trait CryptoHandler: Send + Sync + 'static {
    type CryptoConnectionInstance: CryptoHandlerConnectionInstance;

    fn instantiate(
        &self,
        remote_signature: [u8; 16],
        own_signature: [u8; 16],
        _: &[u8],
        substream_count: u8,
    ) -> (Vec<u8>, Self::CryptoConnectionInstance);

    fn sign_pre_handshake(&self, packet: &mut PRUDPPacket);
}

impl Deref for ExternalConnection{
    type Target = SendingConnection;
    fn deref(&self) -> &Self::Target {
        &self.sending
    }
}

impl Deref for SendingConnection{
    type Target = CommonConnection;
    fn deref(&self) -> &Self::Target {
        &self.common
    }
}

impl ExternalConnection{
    pub async fn recv(&mut self) -> Option<Vec<u8>>{
        self.data_receiver.recv().await
    }
    //todo: make this an actual result instead of an option

    pub fn duplicate_sender(&self) -> SendingConnection{
        self.sending.clone()
    }
}

impl SendingConnection{
    pub async fn send(&self, data: Vec<u8>) -> Option<()> {
        let internal = self.inernal.upgrade()?;

        let mut internal = internal.lock().await;

        internal.send_data_packet(data).await;
        Some(())
    }
}

pub struct Unsecure(pub &'static str);

pub struct UnsecureInstance {
    key: &'static str,
    streams: Vec<EncryptionPair<Rc4<U5>>>,
    self_signature: [u8; 16],
    remote_signature: [u8; 16],
}

// my hand was forced to use lazy so that we can guarantee this code
// only runs once and so that i can put it here as a "constant" (for performance and readability)
// since for some reason rust crypto doesn't have any const time key initialization
static DEFAULT_KEY: Lazy<Key<U5>> = Lazy::new(|| Key::from(*b"CD&ML"));

impl CryptoHandler for Unsecure {
    type CryptoConnectionInstance = UnsecureInstance;

    fn instantiate(
        &self,
        remote_signature: [u8; 16],
        self_signature: [u8; 16],
        _: &[u8],
        substream_count: u8,
    ) -> (Vec<u8>, Self::CryptoConnectionInstance) {
        (
            Vec::new(),
            UnsecureInstance {
                streams: (0..substream_count)
                    .map(|_| EncryptionPair::init_both(|| Rc4::new(&DEFAULT_KEY)))
                    .collect(),
                key: self.0,
                remote_signature,
                self_signature,
            },
        )
    }

    fn sign_pre_handshake(&self, packet: &mut PRUDPPacket) {
        packet.set_sizes();
        packet.calculate_and_assign_signature(self.0, None, None);
    }
}

impl CryptoHandlerConnectionInstance for UnsecureInstance {
    type Encryption = Rc4<U5>;

    fn decrypt_incoming(&mut self, substream: u8, data: &mut [u8]) {
        if let Some(crypt_pair) = self.streams.get_mut(substream as usize){
            crypt_pair.recv.apply_keystream(data);
        }
    }

    fn encrypt_outgoing(&mut self, substream: u8, data: &mut [u8]) {
        if let Some(crypt_pair) = self.streams.get_mut(substream as usize){
            crypt_pair.send.apply_keystream(data);
        }
    }

    fn get_user_id(&self) -> u32 {
        0
    }

    fn sign_connect(&self, packet: &mut PRUDPPacket) {
        packet.set_sizes();
        packet.calculate_and_assign_signature(self.key, None, Some(self.self_signature));
    }

    fn sign_packet(&self, packet: &mut PRUDPPacket) {
        packet.set_sizes();
        packet.calculate_and_assign_signature(self.key, None, Some(self.self_signature));
    }

    fn verify_packet(&self, packet: &PRUDPPacket) -> bool {
        true
    }
}
