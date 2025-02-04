use crate::prudp::socket::ConnectionData;

pub mod auth;
pub mod server;
pub mod secure;
pub mod matchmake_extension;
pub mod matchmake_common;

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