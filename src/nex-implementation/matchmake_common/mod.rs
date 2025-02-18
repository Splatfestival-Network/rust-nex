use std::collections::{BTreeMap};
use std::sync::Arc;
use log::error;
use rand::random;
use tokio::sync::{Mutex, RwLock};
use crate::kerberos::KerberosDateTime;
use crate::protocols::notification::Notification;
use crate::rmc::structures::matchmake::{Gathering, MatchmakeParam, MatchmakeSession};
use crate::rmc::structures::variant::Variant;

#[derive(Default, Debug)]
pub struct ExtendedMatchmakeSession{
    pub session: MatchmakeSession,
    pub connected_players: Vec<Arc<Mutex<ConnectionData>>>,
}

pub struct MatchmakeData{
    pub(crate) matchmake_sessions: BTreeMap<u32, Arc<Mutex<ExtendedMatchmakeSession>>>
}

impl ExtendedMatchmakeSession{
    pub async fn from_matchmake_session(session: MatchmakeSession, host: &Mutex<ConnectionData>) -> Self{
        let host = host.lock().await;

        let ConnectionData{
            active_connection_data,
            ..
        } = &*host;

        let Some(active_connection_data) = active_connection_data else{
            return Default::default();
        };

        let ActiveConnectionData{
            active_secure_connection_data,
            ..
        } = active_connection_data;

        let Some(active_secure_connection_data) = active_secure_connection_data else{
            return Default::default();
        };


        let mm_session = MatchmakeSession{
            gathering: Gathering{
                self_gid: 1,
                owner_pid: active_secure_connection_data.pid,
                host_pid: active_secure_connection_data.pid,
                ..session.gathering.clone()
            },
            datetime: KerberosDateTime::now(),
            session_key: (0..32).map(|_| random()).collect(),
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

    pub async fn add_player(&mut self, socket: &SocketData, conn: Arc<Mutex<ConnectionData>>, join_msg: String) {
        let locked = conn.lock().await;

        let Some(joining_pid) = locked.active_connection_data.as_ref()
            .map(|c|
                c.active_secure_connection_data.as_ref()
                    .map(|c| c.pid)
            ).flatten() else {
            error!("tried to add player without secure connection");
            return
        };

        drop(locked);

        self.connected_players.push(conn);
        self.session.participation_count = self.connected_players.len() as u32;


        for other_connection in &self.connected_players{
            let mut conn = other_connection.lock().await;


            let Some(other_pid) = conn.active_connection_data.as_ref()
                .map(|c|
                    c.active_secure_connection_data.as_ref()
                        .map(|c| c.pid
                        )
                ).flatten() else {
                error!("tried to send connection notification to player secure connection");
                return
            };

            /*if other_pid == self.session.gathering.owner_pid &&
                joining_pid == self.session.gathering.owner_pid{
                continue;
            }*/

            conn.send_notification(socket, Notification{
                pid_source: joining_pid,
                notif_type: 3001,
                param_1: self.session.gathering.self_gid,
                param_2: other_pid,
                str_param: join_msg.clone(),
                param_3: self.session.participation_count
            }).await;
        }
    }
}
pub async fn add_matchmake_session(mm_data: Arc<RwLock<MatchmakeData>>,session: ExtendedMatchmakeSession) -> Arc<Mutex<ExtendedMatchmakeSession>> {
    let gid = session.session.gathering.self_gid;

    let mut mm_data = mm_data.write().await;

    let session = Arc::new(Mutex::new(session));

    mm_data.matchmake_sessions.insert(gid, session.clone());

    session
}

impl MatchmakeData {



    pub async fn try_find_session_with_criteria(&self, ) -> Option<Arc<Mutex<ExtendedMatchmakeSession>>>{
        None
    }
}