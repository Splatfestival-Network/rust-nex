use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use crate::protocols::matchmake_common::MatchmakeData;
use crate::prudp::socket::{ConnectionData, SocketData};
use crate::rmc::message::RMCMessage;
use crate::rmc::response::{ErrorCode, RMCResponseResult};
use crate::rmc::structures::qresult::QResult;
use crate::rmc::structures::RmcSerialize;

pub async fn unregister_gathering(rmcmessage: &RMCMessage, gid: u32, data: Arc<RwLock<MatchmakeData>>) -> RMCResponseResult{
    let mut rd = data.write().await;

    rd.matchmake_sessions.remove(&gid);

    let result = QResult::success(ErrorCode::Core_Unknown);

    let mut response = Vec::new();

    result.serialize(&mut response).expect("aaa");

    rmcmessage.success_with_data(response)
}

pub async fn unregister_gathering_raw_params(rmcmessage: &RMCMessage, _: &Arc<SocketData>, _: &Arc<Mutex<ConnectionData>>, data: Arc<RwLock<MatchmakeData>>) -> RMCResponseResult{
    let mut reader = Cursor::new(&rmcmessage.rest_of_data);

    let Ok(gid) = u32::deserialize(&mut reader) else {
        return rmcmessage.error_result_with_code(ErrorCode::Core_InvalidArgument);
    };




    unregister_gathering(rmcmessage, gid, data).await
}