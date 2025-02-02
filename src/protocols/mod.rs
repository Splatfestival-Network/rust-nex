pub mod auth;
pub mod server;
#[macro_export]
macro_rules! define_protocol {
    ($id:literal ($($varname:ident : $ty:ty),*) => {$($func_id:literal => $func:path),*} ) => {
        #[allow(unused_parens)]
        async fn protocol (rmcmessage: &RMCMessage, $($varname : $ty),*) -> Option<RMCResponse>{
            if rmcmessage.protocol_id != $id{
                return None;
            }

            let self_data: ( $( $ty ),* ) = ($( $varname ),*);

            let response_result = match rmcmessage.method_id{
                $(
                    $func_id => $func ( rmcmessage, self_data).await,
                )*
                _ => {
                    error!("invalid method id sent to protocol {}: {:?}", $id, rmcmessage.method_id);
                    return Some(
                        RMCResponse{
                            protocol_id: $id,
                            response_result: rmcmessage.error_result_with_code(ErrorCode::Core_NotImplemented)
                        }
                    );
                }
            };

            Some(RMCResponse{
                protocol_id: $id,
                response_result
            })
        }
        #[allow(unused_parens)]
        pub fn bound_protocol($($varname : $ty,)*) -> Box<dyn for<'message_lifetime> Fn(&'message_lifetime RMCMessage) -> ::std::pin::Pin<Box<dyn ::std::future::Future<Output = Option<RMCResponse>> + Send + 'message_lifetime>> + Send + Sync>{
            Box::new(
                move |v| {
                    Box::pin(async move {
                        $(
                        let $varname = $varname.clone();
                        )*
                        protocol(v, $($varname,)*).await
                    })
                }
            )
        }
    };
}