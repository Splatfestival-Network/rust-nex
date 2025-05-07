use crate::prudp::packet::flags::{ACK, HAS_SIZE, MULTI_ACK, NEED_ACK, RELIABLE};
use crate::prudp::packet::types::{CONNECT, DATA, DISCONNECT, PING, SYN};
use crate::prudp::packet::PacketOption::{ConnectionSignature, FragmentId, InitialSequenceId, MaximumSubstreamId, SupportedFunctions};
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
use crate::nex::account::Account;
// due to the way this is designed crashing the router thread causes deadlock, sorry ;-;
// (maybe i will fix that some day)

/// PRUDP Socket for accepting connections to then send and recieve data from those clients


pub struct EncryptionPair<T: StreamCipher + Send> {
    pub send: T,
    pub recv: T,
}

impl<T: StreamCipher + Send> EncryptionPair<T> {
    pub fn init_both<F: Fn() -> T>(func: F) -> Self {
        Self {
            recv: func(),
            send: func(),
        }
    }
}

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
        let prev_val = self.reliable_server_counter;
        let (val, _) = self.reliable_server_counter.overflowing_add(1);
        self.reliable_server_counter = val;
        println!("{}", prev_val);
        prev_val
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

        let mut vec = Vec::new();

        packet
            .write_to(&mut vec)
            .expect("somehow failed to convert backet to bytes");

        println!("{}", hex::encode(&vec));

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

        self.socket
            .send_to(&vec, dest.regular_socket_addr)
            .await
            .expect("failed to send data back");
    }

    async fn handle_syn(&self, address: PRUDPSockAddr, packet: PRUDPPacket) {
        info!("got syn");

        let mut response = packet.base_response_packet();

        response.header.types_and_flags.set_types(SYN);
        response.header.types_and_flags.set_flag(ACK);
        response.header.types_and_flags.set_flag(HAS_SIZE);

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

        response.set_sizes();

        self.crypto_handler.sign_pre_handshake(&mut response);

        //println!("got syn: {:?}", response);

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

        let Some((return_data, crypto)) = self.crypto_handler.instantiate(
            remote_signature,
            *own_signature,
            &packet.payload,
            1 + *max_substream,
        ) else {
            error!("someone attempted to connect with invalid data");
            return;
        };

        let mut response = packet.base_response_packet();
        response.header.types_and_flags.set_types(CONNECT);
        response.header.types_and_flags.set_flag(ACK);
        response.header.types_and_flags.set_flag(HAS_SIZE);

        response.header.session_id = session_id;
        response.header.sequence_id = 1;

        response.payload = return_data;


        //let remote_signature = address.calculate_connection_signature();

        response
            .options
            .push(ConnectionSignature(Default::default()));

        for option in &packet.options {
            match option {
                MaximumSubstreamId(max_substream) => response
                    .options
                    .push(MaximumSubstreamId(*max_substream)),
                SupportedFunctions(funcs) => {
                    response.options.push(SupportedFunctions(*funcs))
                }
                _ => { /* ? */ }
            }
        }


        response.set_sizes();

        crypto.sign_connect(&mut response);

        //println!("connect out: {:?}", response);

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

        let mut response = packet.base_acknowledgement_packet();
        response.header.types_and_flags.set_flag(HAS_SIZE | ACK);
        response.header.session_id = conn.session_id;

        conn.crypto_handler_instance.sign_packet(&mut response);

        self.send_packet_unbuffered(address, response).await;

        conn.data_sender.send(data).await.ok();


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

                if packet.header.types_and_flags.get_types() == SYN{
                    println!("Syn: {:?}", packet);
                }

                if packet.header.types_and_flags.get_types() == CONNECT{
                    println!("Connect: {:?}", packet);
                }

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
            DISCONNECT => self.handle_disconnect(address, packet).await,
            PING => self.handle_ping(address, packet).await,
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
                MaximumSubstreamId(0),
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
                MaximumSubstreamId(0),
                ConnectionSignature(remote_signature)
            ],
            ..Default::default()
        };

        self.send_packet_unbuffered(address, packet).await;

        let Some(connect_ack_packet) = recv.recv().await else{
            error!("what");
            return None;
        };

        let (_, crypt) = self.crypto_handler.instantiate(remote_signature, *own_signature, &[], 1)?;

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
    ) -> Option<(Vec<u8>, Self::CryptoConnectionInstance)>;

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
        println!("{}", hex::encode(&data));
        let internal = self.inernal.upgrade()?;

        let mut internal = internal.lock().await;

        internal.send_data_packet(data).await;
        Some(())
    }
}