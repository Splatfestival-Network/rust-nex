use std::io::Cursor;
use std::sync::Arc;
use log::error;
use tokio::sync::Mutex;
use crate::endianness::{IS_BIG_ENDIAN, ReadExtensions};
use crate::rmc::message::RMCMessage;
use crate::rmc::response::{RMCResponseResult};
use crate::rmc::response::ErrorCode::Core_InvalidArgument;
use crate::rmc::structures::{qbuffer, RmcSerialize};
use crate::rmc::structures::qbuffer::QBuffer;

pub async fn send_report(rmcmessage: &RMCMessage, report_id: u32, data: Vec<u8>) -> RMCResponseResult{
    let result = tokio::fs::write(format!("./reports/{}", report_id), data).await;

    match result{
        Ok(_) => {},
        Err(e) => error!("{}", e)
    }

    rmcmessage.success_with_data(Vec::new())
}

pub async fn send_report_raw_params(rmcmessage: &RMCMessage, _: &Arc<SocketData>, _conn_data: &Arc<Mutex<ConnectionData>>, _: ()) -> RMCResponseResult{
    let mut reader = Cursor::new(&rmcmessage.rest_of_data);

    let Ok(error_id) = reader.read_struct(IS_BIG_ENDIAN) else {
        return rmcmessage.error_result_with_code(Core_InvalidArgument);
    };

    let Ok(QBuffer(data)) = QBuffer::deserialize(&mut reader) else {
        return rmcmessage.error_result_with_code(Core_InvalidArgument);
    };

    send_report(rmcmessage, error_id, data).await
}