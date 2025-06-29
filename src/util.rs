use std::ops::Deref;
use std::sync::Arc;
use log::{error, info};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Notify;
use tokio::task;
use rust_nex::reggie::{UnitPacketRead, UnitPacketWrite};

#[derive(Clone)]
pub struct SendingBufferConnection(Sender<Vec<u8>>, Arc<Notify>);

pub struct SplittableBufferConnection(SendingBufferConnection, Receiver<Vec<u8>>);

impl AsRef<SendingBufferConnection> for SplittableBufferConnection{
    fn as_ref(&self) -> &SendingBufferConnection {
        &self.0
    }
}

impl Deref for SplittableBufferConnection{
    type Target = SendingBufferConnection;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}



impl<T: Send + Unpin + AsyncWrite + AsyncRead + 'static> From<T> for SplittableBufferConnection{
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl SplittableBufferConnection {
    fn new<T: Send + Unpin + AsyncWrite + AsyncRead + 'static>(stream: T) -> Self {
        let (outside_send, inside_recv) = channel::<Vec<u8>>(10);
        let (inside_send, outside_recv) = channel::<Vec<u8>>(10);

        let notify = Arc::new(Notify::new());

        {
            let notify = notify.clone();
            task::spawn(async move {
                let sender = inside_send;
                let mut recver = inside_recv;
                let mut stream = stream;


                loop {
                    tokio::select! {
                        data = recver.recv() => {
                            let Some(data) = data else {
                                break;
                            };

                            if let Err(e) = stream.send_buffer(&data[..]).await{
                                error!("error sending data to backend: {}", e);
                                break;
                            }
                        },
                        data = stream.read_buffer() => {
                            let data = match data{
                                Ok(d) => d,
                                Err(e) => {
                                    error!("error reveiving data from backend: {}", e);
                                    break;
                                }
                            };

                            if let Err(e) = sender.send(data).await{
                                error!("a send error occurred {}", e);
                                return;
                            }
                        },
                        _ = notify.notified() => {
                            info!("shutting down connection");
                            break;
                        }
                    }
                }
                stream.shutdown().await;
            });
        }

        Self(SendingBufferConnection(outside_send, notify), outside_recv)
    }
}

impl SendingBufferConnection{
    pub async fn send(&self, buffer: Vec<u8>) -> Option<()>{
        self.0.send(buffer).await.ok()
    }
    pub fn is_alive(&self) -> bool{
        !self.0.is_closed()
    }
    pub async fn disconnect(&self) {
        while !self.0.is_closed() {
            self.1.notify_waiters();
            tokio::task::yield_now().await;
        }
    }
}

impl SplittableBufferConnection{
    pub async fn recv(&mut self) -> Option<Vec<u8>>{
        self.1.recv().await
    }

    pub fn duplicate_sender(&self) -> SendingBufferConnection{
        self.0.clone()
    }
}
