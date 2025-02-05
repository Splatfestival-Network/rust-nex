use std::env;
use std::future::Future;
use std::pin::Pin;
use log::warn;
use once_cell::sync::Lazy;
use crate::grpc;
use crate::prudp::socket::ConnectionData;
use crate::rmc::message::RMCMessage;
use crate::rmc::response::{ErrorCode, RMCResponse};


pub mod auth;
pub mod server;
pub mod secure;
pub mod matchmake_extension;
pub mod matchmake_common;


static IS_MAINTENANCE: Lazy<bool> = Lazy::new(|| {
    env::var("IS_MAINTENANCE")
        .ok()
        .map(|v| v.parse().expect("IS_MAINTENANCE should be a boolean value"))
        .unwrap_or(false)
});
static BYPASS_LEVEL: Lazy<i32> = Lazy::new(|| {
    env::var("MAINTENANCE_BYPASS_MINIMUM_ACCESS_LEVEL")
        .ok()
        .map(|v| v.parse().expect("IS_MAINTENANCE should be a boolean value"))
        .unwrap_or(3)
});


pub fn block_if_maintenance<'a>(rmcmessage: &'a RMCMessage, conn: &'a mut ConnectionData) -> Pin<Box<(dyn Future<Output=Option<RMCResponse>> + Send + 'a)>> {
    Box::pin(async move {
        if let Some(active_conn) = conn.active_connection_data.as_ref() {
            if let Some(secure_conn) = active_conn.active_secure_connection_data.as_ref() {
                if let Ok(mut client) = grpc::account::Client::new().await {
                    if let Ok(client_data) = client.get_user_data(secure_conn.pid).await{
                        if client_data.access_level >= *BYPASS_LEVEL{
                            return None;
                        }
                    }
                }
            }
        }


        warn!("login attempted whilest servers are in maintenance");

        if *IS_MAINTENANCE {
            Some(RMCResponse {
                protocol_id: rmcmessage.protocol_id as u8,
                response_result: rmcmessage.error_result_with_code(ErrorCode::RendezVous_GameServerMaintenance),
            })
        } else {
            None
        }
    })
}

#[macro_export]
macro_rules! define_protocol {
    ($id:literal ($($varname:ident : $ty:ty),*) => {$($func_id:literal => $func:path),*} ) => {
        #[allow(unused_parens)]
        async fn protocol (rmcmessage: &crate::RMCMessage, connection: &mut crate::protocols::ConnectionData, $($varname : $ty),*) -> Option<crate::rmc::response::RMCResponse>{
            if rmcmessage.protocol_id != $id{
                return None;
            }

            let self_data: ( $( $ty ),* ) = ($( $varname ),*);

            let response_result = match rmcmessage.method_id{
                $(
                    $func_id => $func ( rmcmessage, connection, self_data).await,
                )*
                _ => {
                    log::error!("invalid method id sent to protocol {}: {:?}", $id, rmcmessage.method_id);
                    return Some(
                        crate::rmc::response::RMCResponse{
                            protocol_id: $id,
                            response_result: rmcmessage.error_result_with_code(crate::rmc::response::ErrorCode::Core_NotImplemented)
                        }
                    );
                }
            };

            Some(crate::rmc::response::RMCResponse{
                protocol_id: $id,
                response_result
            })
        }
        #[allow(unused_parens)]
        pub fn bound_protocol($($varname : $ty,)*) -> Box<dyn for<'message_lifetime> Fn(&'message_lifetime crate::RMCMessage, &'message_lifetime mut crate::protocols::ConnectionData)
            -> ::std::pin::Pin<Box<dyn ::std::future::Future<Output = Option<crate::rmc::response::RMCResponse>> + Send + 'message_lifetime>> + Send + Sync>{
            Box::new(
                move |v, cd| {
                    Box::pin({
                        $(
                        let $varname = $varname.clone();
                        )*

                        async move {
                            $(
                            let $varname = $varname.clone();
                            )*
                            protocol(v, cd, $($varname,)*).await
                        }
                    })
                }
            )
        }
    };
}