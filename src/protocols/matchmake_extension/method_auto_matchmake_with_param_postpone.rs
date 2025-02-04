use std::sync::Arc;
use tokio::sync::Mutex;
use crate::protocols::matchmake_common::MatchmakeData;
use crate::prudp::socket::ConnectionData;
use crate::rmc::message::RMCMessage;

pub async fn auto_matchmake_with_param_postpone_raw_params(rmcmessage: &RMCMessage, _: &mut ConnectionData, data: Arc<Mutex<MatchmakeData>>){

}