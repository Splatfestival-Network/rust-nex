use std::collections::HashMap;
use std::sync::{Arc, Weak};
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::{Relaxed, Release};
use rand::random;
use tokio::sync::{Mutex, RwLock};
use crate::kerberos::KerberosDateTime;
use crate::nex::user::User;
use crate::rmc::protocols::notifications::{NotificationEvent, RemoteNotification};
use crate::rmc::structures::matchmake::{Gathering, MatchmakeParam, MatchmakeSession};
use crate::rmc::structures::variant::Variant;

pub struct MatchmakeManager{
    pub gid_counter: AtomicU32,
    pub sessions: RwLock<HashMap<u32, Arc<Mutex<ExtendedMatchmakeSession>>>>,
    pub rv_cid_counter: AtomicU32,
    pub users: RwLock<HashMap<u32, Weak<User>>>
}

impl MatchmakeManager{
    pub fn next_gid(&self) -> u32{
        self.gid_counter.fetch_add(1, Relaxed)
    }

    pub fn next_cid(&self) -> u32{
        self.rv_cid_counter.fetch_add(1, Relaxed)
    }
}


#[derive(Default, Debug)]
pub struct ExtendedMatchmakeSession{
    pub session: MatchmakeSession,
    pub connected_players: Vec<Weak<User>>,
}

impl ExtendedMatchmakeSession{
    pub async fn from_matchmake_session(gid: u32, session: MatchmakeSession, host: &Weak<User>) -> Self{
        let Some(host) = host.upgrade() else{
            return Default::default();
        };


        let mm_session = MatchmakeSession{
            gathering: Gathering{
                self_gid: 1,
                owner_pid: host.pid,
                host_pid: host.pid,
                ..session.gathering.clone()
            },
            datetime: KerberosDateTime::now(),
            session_key: vec![16, 118, 112, 238, 158, 122, 106, 219, 196, 238, 34, 21, 228, 127, 137, 75, 198, 215, 192, 113, 84, 157, 53, 144, 210, 99, 233, 179, 232, 113, 203, 64],//(0..32).map(|_| random()).collect(),
            matchmake_param: MatchmakeParam{
                params: vec![
                    ("@SR".to_owned(), Variant::Bool(true)),
                    ("@GIR".to_owned(), Variant::SInt64(3))
                ]
            },
            system_password_enabled: false,
            ..session
        };

        Self{
            session: mm_session,
            connected_players: Default::default()
        }
    }

    pub async fn add_player(&mut self, conn: Weak<User>, join_msg: String) {
        let Some(arc_conn) = conn.upgrade() else {
            return
        };

        let joining_pid = arc_conn.pid;

        let old_particip = self.connected_players.clone();

        self.connected_players.push(conn);
        self.session.participation_count = self.connected_players.len() as u32;


        for other_connection in &self.connected_players{
            let Some(other_conn) = other_connection.upgrade() else {
                continue;
            };


            let other_pid = other_conn.pid;
            /*if other_pid == self.session.gathering.owner_pid &&
                joining_pid == self.session.gathering.owner_pid{
                continue;
            }*/

            other_conn.remote.process_notification_event(NotificationEvent{
                pid_source: joining_pid,
                notif_type: 3001,
                param_1: self.session.gathering.self_gid,
                param_2: other_pid,
                str_param: join_msg.clone(),
                param_3: self.connected_players.len() as _
            }).await;
        }

        for old_conns in &old_particip{
            let Some(old_conns) = old_conns.upgrade() else {
                continue;
            };


            let older_pid = old_conns.pid;

            arc_conn.remote.process_notification_event(NotificationEvent{
                pid_source: joining_pid,
                notif_type: 3001,
                param_1: self.session.gathering.self_gid,
                param_2: older_pid,
                str_param: join_msg.clone(),
                param_3: self.connected_players.len() as _
            }).await;
        }
    }
}