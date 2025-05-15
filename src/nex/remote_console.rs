use macros::rmc_struct;
use crate::rmc::protocols::notifications::{Notification, NotificationEvent, RawNotification, RawNotificationInfo, RemoteNotification};
use crate::rmc::protocols::nat_traversal::{NatTraversalConsole, RemoteNatTraversalConsole, RawNatTraversalConsoleInfo, RawNatTraversalConsole};
use crate::define_rmc_proto;
use crate::nex::user::RemoteUserProtocol;

define_rmc_proto!(
    proto Console{
        Notification,
        NatTraversalConsole
    }
);
/*
#[rmc_struct(Console)]
pub struct TestRemoteConsole{
    pub remote: RemoteUserProtocol,
}

impl Notification for TestRemoteConsole{
    async fn process_notification_event(&self, event: NotificationEvent) {
        println!("NOTIF RECIEVED: {:?}", event);
    }
}*/