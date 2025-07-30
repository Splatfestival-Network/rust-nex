use std::io::Cursor;
use rust_nex::rmc::structures::RmcSerialize;
use rust_nex::reggie::{RemoteEdgeNodeHolder, UnitPacketRead};
use std::net::SocketAddrV4;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use log::{error, info};
use tokio::net::{TcpListener, TcpStream};
use tokio::task;
use rust_nex::common::setup;
use rust_nex::executables::common::{OWN_IP_PRIVATE, SECURE_EDGE_NODE_HOLDER, SERVER_PORT};
use rust_nex::nex::matchmake::MatchmakeManager;
use rust_nex::nex::remote_console::RemoteConsole;
use rust_nex::nex::user::User;
use rust_nex::reggie::EdgeNodeHolderConnectOption::DontRegister;
use rust_nex::rmc::protocols::{new_rmc_gateway_connection, OnlyRemote};
use rust_nex::rnex_proxy_common::ConnectionInitData;
use rust_nex::rmc::protocols::RemoteInstantiatable;
use rust_nex::util::SplittableBufferConnection;

#[tokio::main]
async fn main() {
    setup();

    let listen = TcpListener::bind(SocketAddrV4::new(*OWN_IP_PRIVATE, *SERVER_PORT)).await.unwrap();

    let mmm = Arc::new(MatchmakeManager{
        gid_counter: AtomicU32::new(1),
        sessions: Default::default(),
        users: Default::default(),
        rv_cid_counter: AtomicU32::new(1),
    });

    let weak_mmm = Arc::downgrade(&mmm);

    MatchmakeManager::initialize_garbage_collect_thread(weak_mmm).await;

    while let Ok((mut stream, addr)) = listen.accept().await {
        let buffer = match stream.read_buffer().await{
            Ok(v) => v,
            Err(e) => {
                error!("an error ocurred whilest reading connection data buffer: {:?}", e);
                continue;
            }
        };

        let user_connection_data = ConnectionInitData::deserialize(&mut Cursor::new(buffer));

        let user_connection_data = match user_connection_data{
            Ok(v) => v,
            Err(e) => {
                error!("an error ocurred whilest reading connection data: {:?}", e);
                continue;
            }
        };


        let mmm = mmm.clone();
        task::spawn(async move {
            info!("connection to secure backend established");
            new_rmc_gateway_connection(stream.into(), |r| {
                Arc::new_cyclic(|this| User{
                    this: this.clone(),
                    ip: user_connection_data.prudpsock_addr,
                    pid: user_connection_data.pid,
                    remote: RemoteConsole::new(r),
                    matchmake_manager: mmm,
                    station_url: Default::default()
                })
            });
        });

    }
}