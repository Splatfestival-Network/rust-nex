pub mod auth;
pub mod server;
#[macro_export]
macro_rules! define_protocol {
    ($id:literal => {$($func_id:literal => $func:path),*} ) => {
        pub fn protocol(rmcmessage: &RMCMessage) -> Option<RMCResponse>{
            if rmcmessage.protocol_id != $id{
                return None;
            }

            let response_result = match rmcmessage.method_id{
                $(
                    $func_id => $func(rmcmessage),
                ),*
                _ => {
                    error!("invalid method id sent to protocol {}: {:?}", $id, rmcmessage.method_id);
                    rmcmessage.error_result_with_code(ErrorCode::Core_NotImplemented)
                }
            };

            Some(RMCResponse{
                protocol_id: $id,
                response_result
            })
        }
    };
}