use std::io::{Cursor, Write};
use std::sync::Arc;
use bytemuck::bytes_of;
use tokio::sync::Mutex;
use crate::prudp::station_url::{nat_types, StationUrl};
use crate::prudp::station_url::Type::PRUDPS;
use crate::prudp::station_url::UrlOptions::{Address, NatFiltering, NatMapping, NatType, Port, PrincipalID, RVConnectionID};
use crate::rmc::message::RMCMessage;
use crate::rmc::response::{ErrorCode, RMCResponseResult};
use crate::rmc::structures::qresult::QResult;
use crate::rmc::structures::RmcSerialize;

type StringList = Vec<String>;

pub async fn register(rmcmessage: &RMCMessage, _station_urls: Vec<StationUrl>, conn_data: &Arc<Mutex<ConnectionData>>) -> RMCResponseResult{
    let locked = conn_data.lock().await;
    let Some(active_connection_data) = locked.active_connection_data.as_ref() else {
        return rmcmessage.error_result_with_code(ErrorCode::RendezVous_NotAuthenticated)
    };

    let Some(active_secure_connection_data) = active_connection_data.active_secure_connection_data.as_ref() else {
        return rmcmessage.error_result_with_code(ErrorCode::RendezVous_NotAuthenticated)
    };

    let public_station = StationUrl{
        url_type: PRUDPS,
        options: vec![
            RVConnectionID(active_connection_data.connection_id),
            Address(*locked.sock_addr.regular_socket_addr.ip()),
            Port(locked.sock_addr.regular_socket_addr.port()),
            NatFiltering(0),
            NatMapping(0),
            NatType(nat_types::BEHIND_NAT),
            PrincipalID(active_secure_connection_data.pid),
        ]
    };



    let result = QResult::success(ErrorCode::Core_Unknown);

    let mut response = Vec::new();

    result.serialize(&mut response).expect("unable to serialize result");
    response.write_all(bytes_of(&active_connection_data.connection_id)).expect("unable to serialize connection id");
    public_station.to_string().serialize(&mut response).expect("unable to serialize station id");

    rmcmessage.success_with_data(response)
}

pub async fn register_raw_params(rmcmessage: &RMCMessage, _: &Arc<SocketData>, conn_data: &Arc<Mutex<ConnectionData>>, _: ()) -> RMCResponseResult{
    let mut reader = Cursor::new(&rmcmessage.rest_of_data);

    let Ok(station_urls) = StringList::deserialize(&mut reader) else {
        return rmcmessage.error_result_with_code(ErrorCode::Core_InvalidArgument);
    };

    let Ok(station_urls): Result<Vec<StationUrl>, _> = station_urls.iter().map(|c| StationUrl::try_from((&c) as &str)).collect() else {
        return rmcmessage.error_result_with_code(ErrorCode::Core_InvalidArgument);
    };

    register(rmcmessage, station_urls, conn_data).await
}