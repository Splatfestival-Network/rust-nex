use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tokio::time::sleep;
use crate::protocols::matchmake_common::MatchmakeData;
use crate::rmc::message::RMCMessage;
use crate::rmc::response::{ErrorCode, RMCResponseResult};
use crate::rmc::structures::matchmake::CreateMatchmakeSessionParam;

pub async fn report_nat_properties(
    rmcmessage: &RMCMessage,
    socket: &Arc<SocketData>,
    connection_data: &Arc<Mutex<ConnectionData>>,
) -> RMCResponseResult{
    sleep(Duration::from_millis(50)).await;
    rmcmessage.success_with_data(Vec::new())
}

pub async fn report_nat_properties_raw_params(
    rmcmessage: &RMCMessage,
    socket: &Arc<SocketData>,
    connection_data: &Arc<Mutex<ConnectionData>>,
    _: ()
) -> RMCResponseResult{
    let mut reader = Cursor::new(&rmcmessage.rest_of_data);

    report_nat_properties(rmcmessage, socket, connection_data).await
}