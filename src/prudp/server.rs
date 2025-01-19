use std::{env, io, thread};
use std::io::Cursor;
use std::marker::PhantomData;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use once_cell::sync::Lazy;
use log::error;
use crate::prudp::auth_module::AuthModule;
use crate::prudp::endpoint::Endpoint;
use crate::prudp::packet::PRUDPPacket;

static SERVER_DATAGRAMS: Lazy<u8> = Lazy::new(||{
    env::var("SERVER_DATAGRAM_COUNT").ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
});

pub struct NexServer{
    pub endpoints: Mutex<Vec<Endpoint>>,
    pub running: AtomicBool,
    //pub auth_module: Arc<dyn AuthModule>
    _no_outside_construction: PhantomData<()>
}

impl NexServer{
    fn process_prudp_packet(&self, packet: &PRUDPPacket){

    }
    fn process_prudp_packets(&self, addr: Ipv4Addr, udp_message: &[u8]){
        let mut stream = Cursor::new(udp_message);

        while stream.position() as usize != udp_message.len() {
            let packet = match PRUDPPacket::new(&mut stream){
                Ok(p) => p,
                Err(e) => {
                    error!("Somebody is fucking with the servers or their connection is bad(from {})", addr);
                    break;
                },
            };


        }
    }

    fn server_thread_entry(self: Arc<Self>, socket: Arc<UdpSocket>){
        while self.running.load(Ordering::Relaxed) {
            // yes we actually allow the max udp to be read lol
            let mut msg_buffer = vec![0u8; 65507];

            let (len, addr) = socket.recv_from(&mut msg_buffer)
                .expect("Datagram thread crashed due to unexpected error from recv_from");

            let current_msg = &msg_buffer[0..len];


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

