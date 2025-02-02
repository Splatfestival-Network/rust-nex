use tonic::{Request, Status};

type InterceptorFunc = Box<(dyn Fn(Request<()>) -> Result<Request<()>, Status> + Send)>;
mod protobufs;
pub mod account;