use std::{env, io};
use std::io::Cursor;
use std::marker::PhantomData;
use tokio::net::UdpSocket;
use std::net::{SocketAddr, SocketAddrV4};
use std::net::SocketAddr::V4;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::task::JoinHandle;
use once_cell::sync::Lazy;
use log::{error, info, trace};
use thiserror::Error;
use tokio::sync::RwLock;
use crate::prudp::socket::{new_socket_pair, AnyInternalSocket, CryptoHandler, ExternalSocket};
use crate::prudp::packet::{PRUDPPacket, VirtualPort};
use crate::prudp::router::Error::VirtualPortTaken;

static SERVER_DATAGRAMS: Lazy<u8> = Lazy::new(||{
    env::var("SERVER_DATAGRAM_COUNT").ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
});

pub struct Router {
    endpoints: RwLock<[Option<Arc<dyn AnyInternalSocket>>; 16]>,
    running: AtomicBool,
    socket: Arc<UdpSocket>,
    _no_outside_construction: PhantomData<()>
}
#[derive(Debug, Error)]
pub enum Error{
    #[error("tried to register socket to a port which is already taken (port: {0})")]
    VirtualPortTaken(u8)
}


impl Router {
    async fn process_prudp_packets<'a>(self: Arc<Self>, _socket: Arc<UdpSocket>, addr: SocketAddrV4, udp_message: Vec<u8>){
        let mut stream = Cursor::new(&udp_message);

        while stream.position() as usize != udp_message.len() {
            let packet = match PRUDPPacket::new(&mut stream){
                Ok(p) => p,
                Err(e) => {
                    error!("Somebody({}) is fucking with the servers or their connection is bad (reason: {})", addr, e);
                    break;
                },
            };

            trace!("got valid prudp packet from someone({}): \n{:?}", addr, packet);

            let connection = packet.source_sockaddr(addr);
            

            let endpoints = self.endpoints.read().await;

            let Some(endpoint) = endpoints[packet.header.destination_port.get_port_number() as usize].as_ref() else {
                error!("connection to invalid endpoint({}) attempted by {}", packet.header.destination_port.get_port_number(), connection.regular_socket_addr);
                continue;
            };

            let endpoint = endpoint.clone();

            // Dont keep the locked structure for too long
            drop(endpoints);

            trace!("sending packet to endpoint");

            tokio::spawn(async move {
                endpoint.recieve_packet(connection, packet).await
            });
        }
    }

    async fn server_thread_send_entry(self: Arc<Self>, socket: Arc<UdpSocket>){
        info!("starting datagram thread");

        while self.running.load(Ordering::Relaxed) {
            // yes we actually allow the max udp to be read lol
            let mut msg_buffer = vec![0u8; 65507];

            let (len, addr) = socket.recv_from(&mut msg_buffer)
                .await.expect("Datagram thread crashed due to unexpected error from recv_from");

            let V4(addr) = addr else {
                error!("somehow got ipv6 packet...? ignoring");
                continue;
            };


            let current_msg = &msg_buffer[0..len];

            tokio::spawn(self.clone().process_prudp_packets(socket.clone(), addr, current_msg.to_vec()));
        }
    }
    
    pub async fn new(addr: SocketAddrV4) -> io::Result<(Arc<Self>, JoinHandle<()>)>{
        trace!("starting router on {}", addr);

        let socket = Arc::new(UdpSocket::bind(addr).await?);

        let own_impl = Router {
            endpoints: Default::default(),
            running: AtomicBool::new(true),
            socket: socket.clone(),
            _no_outside_construction: Default::default()
        };

        let arc = Arc::new(own_impl);


        let task = {
            let socket = socket.clone();
            let server= arc.clone();

            tokio::spawn(async {
                server.server_thread_send_entry(socket).await;
            })
        };

        {
            let _socket = socket.clone();
            let _server = arc.clone();

            tokio::spawn(async {
                //server thread sender entry
                // todo: make this run in the socket cause that makes more sense
                //server.server_thread_recieve_entry(socket).await;
            });
        }


        Ok((arc, task))
    }

    pub fn get_udp_socket(&self) -> Arc<UdpSocket>{
        self.socket.clone()
    }

    // This will remove a socket from the router, this renders all instances of that socket unable
    // to recieve any more data making the error out on trying to for example recieve connections
    pub async fn remove_socket(&self, virtual_port: VirtualPort){
        self.endpoints.write().await[virtual_port.get_port_number() as usize] = None;
    }

    // returns Some(()) i
    pub(crate) async fn add_socket<E: CryptoHandler>(&self, virtual_port: VirtualPort, encryption: E)
        -> Result<ExternalSocket, Error>{
        let mut endpoints = self.endpoints.write().await;

        let idx = virtual_port.get_port_number() as usize;

        // dont create the socket if we dont need to
        if !endpoints[idx].is_none(){
            return Err(VirtualPortTaken(idx as u8));
        }

        let (internal, external) = new_socket_pair(virtual_port, encryption, self.socket.clone());

        endpoints[idx] = Some(internal);

        Ok(external)
    }

    pub fn get_own_address(&self) -> SocketAddrV4{
        match self.socket.local_addr().expect("unable to get socket address"){
            SocketAddr::V4(v4) => v4,
            _ => unreachable!()
        }
    }
}

