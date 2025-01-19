use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};
use crate::prudp::endpoint::Endpoint;

pub struct NexServer{
    pub endpoints: Mutex<Vec<Endpoint>>,
    _no_outside_construction: PhantomData<()>
}

impl NexServer{
    fn server_thread_entry(){
            
    }
    
    pub fn new() -> Arc<Self>{
        let own_impl = NexServer{
            endpoints: Default::default(),
            _no_outside_construction: Default::default()
        };

        let arc = Arc::new(own_impl);
    }
}

#[cfg(test)]
mod test{
    use std::ops::Deref;
    use std::sync::Arc;
    use crate::prudp::server::{NexServer};

    #[test]
    fn test(){
        let server = NexServer::new();

        let a = (server.deref()).clone();

    }
}



