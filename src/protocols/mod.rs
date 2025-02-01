pub mod auth;
pub mod server;
#[macro_export]
macro_rules! define_protocol {
    ($id:literal $( <$lifetime:lifetime> )?($($varname:ident : $ty:ty),*) => {$($func_id:literal => $func:path),*} ) => {
        fn protocol $( <$lifetime> )? (rmcmessage: &RMCMessage, $($varname : $ty,)*) -> Option<RMCResponse>{
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

        pub fn bound_protocol$(<$lifetime>)?($($varname : $ty,)*) -> Box<dyn Fn(&RMCMessage) -> Option<RMCResponse> + Send + Sync $( + $lifetime)?>{
            Box::new(
                move |v| {
                    $(
                    let $varname = $varname.clone();
                    )*
                    protocol(v, $($varname,)*)
                }
            )
        }
    };
}