use std::collections::{BTreeMap};
use std::sync::Arc;
use log::error;
use tokio::sync::Mutex;
use crate::protocols::notification::Notification;
use crate::prudp::socket::{ConnectionData, SocketData};
use crate::rmc::structures::matchmake::MatchmakeSession;

pub struct ExtendedMatchmakeSession{
    pub session: MatchmakeSession,
    pub connected_players: Vec<Arc<Mutex<ConnectionData>>>,
}

pub struct MatchmakeData{
    pub(crate) matchmake_sessions: BTreeMap<u32, Arc<Mutex<ExtendedMatchmakeSession>>>
}

impl ExtendedMatchmakeSession{
    pub async fn add_player(&mut self, socket: &SocketData, conn: Arc<Mutex<ConnectionData>>, join_msg: String) {
        let Some(pid) = conn.lock().await.active_connection_data.as_ref()
            .map(|c|
                c.active_secure_connection_data.as_ref()
                    .map(|c| c.pid
                    )
            ).flatten() else {
            error!("tried to add player without secure connection");
            return
        };

        self.connected_players.push(conn);


        for conn in &self.connected_players{
            let Some(other_pid) = conn.lock().await.active_connection_data.as_ref()
                .map(|c|
                    c.active_secure_connection_data.as_ref()
                        .map(|c| c.pid
                        )
                ).flatten() else {
                error!("tried to send connection notification to player secure connection");
                return
            };

            let mut conn = conn.lock().await;

            conn.send_notification(socket, Notification{
                pid_source: pid,
                notif_type: 3001,
                param_1: self.session.gathering.self_gid,
                param_2: other_pid,
                str_param: join_msg.clone(),
            }).await;


        }
    }
}

impl MatchmakeData {
    pub async fn try_find_session_with_criteria(&self, ) -> Option<Arc<Mutex<ExtendedMatchmakeSession>>>{
        None
    }
}