use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use crate::protocols::matchmake_common::{add_matchmake_session, ExtendedMatchmakeSession, MatchmakeData};
use crate::protocols::matchmake_extension::method_auto_matchmake_with_param_postpone::auto_matchmake_with_param_postpone;
use crate::prudp::socket::{ConnectionData, SocketData};
use crate::rmc::message::RMCMessage;
use crate::rmc::response::{ErrorCode, RMCResponseResult};
use crate::rmc::structures::matchmake::{AutoMatchmakeParam, CreateMatchmakeSessionParam};
use crate::rmc::structures::RmcSerialize;

pub async fn create_matchmake_session_with_param(
    rmcmessage: &RMCMessage,
    conn: &Arc<Mutex<ConnectionData>>,
    socket: &Arc<SocketData>,
    mm_data: Arc<RwLock<MatchmakeData>>,
    create_matchmake_session: CreateMatchmakeSessionParam
) -> RMCResponseResult {
    let locked_conn = conn.lock().await;
    let Some(secure_conn) =
        locked_conn.active_connection_data.as_ref().map(|a| a.active_secure_connection_data.as_ref()).flatten() else {
        return rmcmessage.error_result_with_code(ErrorCode::Core_Exception);
    };

    println!("{:?}", create_matchmake_session);

    let pid = secure_conn.pid;

    let mut session =
        ExtendedMatchmakeSession::from_matchmake_session(create_matchmake_session.matchmake_session, &conn).await;

    session.session.participation_count = create_matchmake_session.participation_count as u32;

    let session = add_matchmake_session(mm_data, session).await;

     let mut session = session.lock().await;

    session.add_player(&socket, conn.clone(), create_matchmake_session.join_message).await;

    let mut response = Vec::new();

    session.session.serialize(&mut response).expect("unable to serialize session");

    rmcmessage.success_with_data(response)
}

pub async fn create_matchmake_session_with_param_raw_params(
    rmcmessage: &RMCMessage,
    socket: &Arc<SocketData>,
    connection_data: &Arc<Mutex<ConnectionData>>,
    data: Arc<RwLock<MatchmakeData>>
) -> RMCResponseResult{
    let mut reader = Cursor::new(&rmcmessage.rest_of_data);

    let Ok(matchmake_param) = CreateMatchmakeSessionParam::deserialize(&mut reader) else {
        return rmcmessage.error_result_with_code(ErrorCode::Core_InvalidArgument);
    };



    create_matchmake_session_with_param(rmcmessage, connection_data, socket, data, matchmake_param).await
}