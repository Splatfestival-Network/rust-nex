use std::env;
use std::net::Ipv4Addr;
use once_cell::sync::Lazy;
use tonic::{Request, Status};

type InterceptorFunc = Box<(dyn Fn(Request<()>) -> Result<Request<()>, Status> + Send)>;
mod protobufs;
pub mod account;