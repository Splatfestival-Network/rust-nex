use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;
use chrono::SecondsFormat::Millis;
use log::info;
use rand::random;
use tokio::sync::{Mutex, RwLock};
use tokio::time::sleep;
use crate::protocols::matchmake_common::{ExtendedMatchmakeSession, MatchmakeData};
use crate::prudp::socket::{ConnectionData, SocketData};
use crate::rmc::message::RMCMessage;
use crate::rmc::response::{ErrorCode, RMCResponseResult};
use crate::rmc::structures::matchmake::{AutoMatchmakeParam, MatchmakeSession};
use crate::rmc::structures::RmcSerialize;



pub async fn auto_matchmake_with_param_postpone(
    rmcmessage: &RMCMessage,
    conn: &Arc<Mutex<ConnectionData>>,
    socket: &Arc<SocketData>,
    mm_data: Arc<RwLock<MatchmakeData>>,
    auto_matchmake_param: AutoMatchmakeParam
) -> RMCResponseResult{
    //println!("auto_matchmake_with_param_postpone: {:?}", auto_matchmake_param);
    let locked_conn = conn.lock().await;
    let Some(secure_conn) =
        locked_conn.active_connection_data.as_ref().map(|a| a.active_secure_connection_data.as_ref()).flatten() else {
        return rmcmessage.error_result_with_code(ErrorCode::Core_Exception);
    };

    let pid = secure_conn.pid;

    drop(locked_conn);

    let mm_data_read = mm_data.read().await;
    //todo: there is a bit of a race condition here, i dont have any idea on how to fix it though...
    let session = if let Some(session) = mm_data_read.try_find_session_with_criteria().await{
        session
    } else {
        // drop it first so that we dont cause a deadlock, also drop it right here so we dont hold
        // up anything else unnescesarily
        drop(mm_data_read);

        let session =
            ExtendedMatchmakeSession::from_matchmake_session(auto_matchmake_param.matchmake_session, &conn).await;

        let gid = session.session.gathering.self_gid;

        let mut mm_data = mm_data.write().await;

        let session = Arc::new(Mutex::new(session));

        mm_data.matchmake_sessions.insert(gid, session.clone());

        session
    };

    let mut locked_session = session.lock().await;

    //todo: refactor so that this works
    {
        let session = session.clone();
        let socket = socket.clone();
        let connection = conn.clone();
        let join_msg = auto_matchmake_param.join_message.clone();
        tokio::spawn(async move{
            sleep(Duration::from_millis(500)).await;
            println!("adding player");
            let mut session = session.lock().await;
            session.add_player(&socket, connection, join_msg).await;
        });
    }

    info!("new session: {:?}", locked_session);

    let mut response = Vec::new();

    locked_session.session.serialize(&mut response).expect("unable to serialize matchmake session");

    rmcmessage.success_with_data(response)
}

pub async fn auto_matchmake_with_param_postpone_raw_params(
    rmcmessage: &RMCMessage,
    socket: &Arc<SocketData>,
    connection_data: &Arc<Mutex<ConnectionData>>,
    data: Arc<RwLock<MatchmakeData>>
) -> RMCResponseResult{
    let mut reader = Cursor::new(&rmcmessage.rest_of_data);

    let Ok(matchmake_param) = AutoMatchmakeParam::deserialize(&mut reader) else {
        return rmcmessage.error_result_with_code(ErrorCode::Core_InvalidArgument);
    };



    auto_matchmake_with_param_postpone(rmcmessage, connection_data, socket, data, matchmake_param).await
}