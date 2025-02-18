use std::net::SocketAddrV4;
use once_cell::sync::Lazy;
use rocket::{get, routes, Rocket};
use rocket::serde::json::Json;
use tokio::task::JoinHandle;
use serde::Serialize;
use tokio::sync::Mutex;

#[get("/")]
async fn server_data() -> Json<WebData> {
    Json(WEB_DATA.lock().await.clone())
}

pub async fn start_web() -> JoinHandle<()>{
    tokio::spawn(async{
        rocket::build()
            .mount("/",routes![server_data])
            .launch().await
            .expect("unable to start webserver");
    })
}
#[derive(Serialize, Clone)]
pub enum DirectionalData{
    Incoming(String),
    Outgoing(String)
}

#[derive(Serialize, Default, Clone)]
pub struct WebData{
    pub data: Vec<(SocketAddrV4, DirectionalData)>
}

pub static WEB_DATA: Lazy<Mutex<WebData>> = Lazy::new(|| Mutex::new(
    WebData{
        data: Vec::new(),
    }
));