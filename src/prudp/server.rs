use std::{env, io, thread};
use std::io::Cursor;
use std::marker::PhantomData;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream, UdpSocket};
use std::net::SocketAddr::V4;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use once_cell::sync::Lazy;
use log::{error, info};
use crate::prudp::auth_module::AuthModule;
use crate::prudp::endpoint::Endpoint;
use crate::prudp::packet::{PRUDPPacket, VirtualPort};
use crate::prudp::sockaddr::PRUDPSockAddr;

static SERVER_DATAGRAMS: Lazy<u8> = Lazy::new(||{
    env::var("SERVER_DATAGRAM_COUNT").ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
});

pub struct NexServer{
    pub endpoints: RwLock<Vec<Endpoint>>,
    pub running: AtomicBool,
    //pub auth_module: Arc<dyn AuthModule>
    _no_outside_construction: PhantomData<()>
}

pub struct Connection<'a>{
    socket: &'a UdpSocket,
    prudp_addr: PRUDPSockAddr
}


impl NexServer{
    fn process_prudp_packet(&self, packet: &PRUDPPacket){

    }
    fn process_prudp_packets<'a>(&self, socket: &'a UdpSocket, addr: SocketAddrV4, udp_message: &[u8]){
        let mut stream = Cursor::new(udp_message);

        while stream.position() as usize != udp_message.len() {
            let packet = match PRUDPPacket::new(&mut stream){
                Ok(p) => p,
                Err(e) => {
                    error!("Somebody({}) is fucking with the servers or their connection is bad", addr);
                    break;
                },
            };

            info!("got valid prudp packet from someone({}): \n{:?}", addr, packet);

            let connection = Connection{
                socket,
                prudp_addr: packet.source_sockaddr(addr)
            };

            let endpoints = self.endpoints.read().expect("poison");

            let Some(endpoint) = endpoints.iter().find(|e|{
                e.get_virual_port().get_port_number() == connection.prudp_addr.virtual_port.get_port_number()
            }) else {
                error!("connection to invalid endpoint({}) attempted by {}", connection.prudp_addr.virtual_port.get_port_number(), connection.prudp_addr.regular_socket_addr);
                continue;
            };





        }
    }

    fn server_thread_entry(self: Arc<Self>, socket: Arc<UdpSocket>){
        info!("starting datagram thread");

        while self.running.load(Ordering::Relaxed) {
            // yes we actually allow the max udp to be read lol
            let mut msg_buffer = vec![0u8; 65507];

            let (len, addr) = socket.recv_from(&mut msg_buffer)
                .expect("Datagram thread crashed due to unexpected error from recv_from");

            let V4(addr) = addr else {
                error!("somehow got ipv6 packet...? ignoring");
                continue;
            };

            let current_msg = &msg_buffer[0..len];
            info!("attempting to process message");
            self.process_prudp_packets(&socket, addr, current_msg);
        }
    }
    
    pub fn new(addr: SocketAddrV4) -> io::Result<(Arc<Self>, JoinHandle<()>)>{
        let own_impl = NexServer{
            endpoints: Default::default(),
            running: AtomicBool::new(true),
            _no_outside_construction: Default::default()
        };

        let arc = Arc::new(own_impl);

        let socket = Arc::new(UdpSocket::bind(addr)?);

        let mut thread = None;

        for _ in 0..*SERVER_DATAGRAMS {
            let socket = socket.clone();
            let server= arc.clone();

            thread = Some(thread::spawn(move || {
                server.server_thread_entry(socket);
            }));
        }

        let thread = thread.expect("cannot have less than 1 thread for a server");


        Ok((arc, thread))
    }
}

