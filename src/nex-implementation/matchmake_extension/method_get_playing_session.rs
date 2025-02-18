use std::io::Cursor;
use std::sync::Arc;
use log::info;
use tokio::sync::{Mutex, RwLock};
use crate::protocols::matchmake_common::MatchmakeData;
use crate::rmc::message::RMCMessage;
use crate::rmc::response::{ErrorCode, RMCResponseResult};
use crate::rmc::structures::RmcSerialize;

type PIDList = Vec<u32>;

async fn get_playing_session(rmcmessage: &RMCMessage, _data: Arc<RwLock<MatchmakeData>>) -> RMCResponseResult {
    //todo: propperly implement this

    let cheeseburger = PIDList::new();

    let mut vec = Vec::new();

    cheeseburger.serialize(&mut vec).expect("somehow unable to write cheeseburger");

    rmcmessage.success_with_data(vec)
}

pub async fn get_playing_session_raw_params(rmcmessage: &RMCMessage, _: &Arc<SocketData>, _: &Arc<Mutex<ConnectionData>>, data: Arc<RwLock<MatchmakeData>>) -> RMCResponseResult{
    let mut reader = Cursor::new(&rmcmessage.rest_of_data);

    let Ok(list) = PIDList::deserialize(&mut reader) else {
        return rmcmessage.error_result_with_code(ErrorCode::FPD_FriendNotExists);
    };

    info!("get_playing_session got called with {:?}", list);

    get_playing_session(rmcmessage, data).await
}