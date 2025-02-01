use std::{env, result};
use std::net::{Ipv4Addr, SocketAddrV4};
use once_cell::sync::Lazy;
use thiserror::Error;
use tonic::codegen::http::uri::InvalidUri;
use tonic::metadata::{Ascii, MetadataValue};
use tonic::{Request, Status, transport};
use tonic::codegen::InterceptedService;
use tonic::service::Interceptor;
use tonic::transport::Channel;
use crate::grpc::{InterceptorFunc, protobufs};
use crate::grpc::protobufs::account::account_client::AccountClient;
use crate::grpc::protobufs::account::GetNexPasswordRequest;

static API_KEY: Lazy<MetadataValue<Ascii>> = Lazy::new(||{
    let key = env::var("ACCOUNT_GRPC_API_KEY")
        .expect("no public ip specified");

    key.parse().expect("unable to parse metadata value")
});

static PORT: Lazy<u16> = Lazy::new(||{
    env::var("ACCOUNT_GRPC_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7071)
});

static IP: Lazy<Ipv4Addr> = Lazy::new(||{
    env::var("ACCOUNT_GRPC_IP")
        .ok()
        .and_then(|s| s.parse().ok())
        .expect("no public ip specified")
});

static CLIENT_URI: Lazy<String> = Lazy::new(||{
    format!("http://{}:{}", *IP, *PORT)
});

#[derive(Error, Debug)]
pub enum Error{
    #[error(transparent)]
    Transport(#[from] transport::Error),
    #[error(transparent)]
    Status(#[from] tonic::Status)
}

pub type Result<T> = result::Result<T, Error>;



pub struct Client(AccountClient<InterceptedService<Channel, InterceptorFunc>>);

impl Client{
    pub async fn new() -> Result<Self>{
        let channel = Channel::from_static(&*CLIENT_URI).connect().await?;

        let func = Box::new(&|mut req: Request<()>|{
            req.metadata_mut().insert("x-api-key", API_KEY.clone());
            Ok(req)
        }) as InterceptorFunc;

        let client = AccountClient::with_interceptor(channel, func);
        Ok(Self(client))
    }

    pub async fn get_nex_password(&mut self , pid: u32) -> Result<Box<str>>{
        let req = Request::new(GetNexPasswordRequest{
            pid
        });

        let response = self.0.get_nex_password(req).await?.into_inner();

        Ok(response.password.into_boxed_str())
    }
}

#[cfg(test)]
mod test{



}