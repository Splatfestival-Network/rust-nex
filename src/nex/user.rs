use std::io::ErrorKind::HostUnreachable;
use crate::define_rmc_proto;
use crate::nex::matchmake::{ExtendedMatchmakeSession, MatchmakeManager};
use crate::nex::remote_console::RemoteConsole;
use crate::prudp::sockaddr::PRUDPSockAddr;
use crate::prudp::station_url::Type::{PRUDP, PRUDPS};
use crate::prudp::station_url::UrlOptions::{
    Address, NatFiltering, NatMapping, NatType, Platform, Port, PrincipalID, RVConnectionID,
    StreamID, PMP, UPNP,
};
use crate::prudp::station_url::{nat_types, StationUrl, Type};
use crate::rmc::protocols::matchmake::{
    Matchmake, RawMatchmake, RawMatchmakeInfo, RemoteMatchmake,
};
use crate::rmc::protocols::matchmake_extension::{
    MatchmakeExtension, RawMatchmakeExtension, RawMatchmakeExtensionInfo, RemoteMatchmakeExtension,
};
use crate::rmc::protocols::nat_traversal::{
    NatTraversal, RawNatTraversal, RawNatTraversalInfo, RemoteNatTraversal,
};
use crate::rmc::protocols::secure::{RawSecure, RawSecureInfo, RemoteSecure, Secure};
use crate::rmc::response::ErrorCode;
use crate::rmc::structures::matchmake::{AutoMatchmakeParam, CreateMatchmakeSessionParam, JoinMatchmakeSessionParam, MatchmakeSession};
use crate::rmc::structures::qresult::QResult;
use macros::rmc_struct;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::{Arc, Weak};
use log::{error, info};
use rocket::http::ext::IntoCollection;
use tokio::sync::{Mutex, RwLock};
use crate::prudp::station_url::nat_types::PUBLIC;
use crate::rmc::response::ErrorCode::{Core_Exception, Core_InvalidArgument, RendezVous_AccountExpired, RendezVous_SessionVoid};

define_rmc_proto!(
    proto UserProtocol{
        Secure,
        MatchmakeExtension,
        Matchmake,
        NatTraversal
    }
);

#[rmc_struct(UserProtocol)]
pub struct User {
    pub pid: u32,
    pub ip: PRUDPSockAddr,
    pub this: Weak<User>,
    pub remote: RemoteConsole,
    pub station_url: RwLock<Vec<StationUrl>>,
    pub matchmake_manager: Arc<MatchmakeManager>,
}

impl Secure for User {
    async fn register(
        &self,
        station_urls: Vec<StationUrl>,
    ) -> Result<(QResult, u32, StationUrl), ErrorCode> {
        let cid = self.matchmake_manager.next_cid();

        println!("{:?}", station_urls);

        let mut users = self.matchmake_manager.users.write().await;
        users.insert(cid, self.this.clone());
        drop(users);

        let mut public_station: Option<StationUrl> = None;
        let mut private_station: Option<StationUrl> = None;

        for station in station_urls{
            let is_public = station.options.iter().any(|v| {
                if let NatType(v) = v {
                    if *v & PUBLIC != 0 {
                        return true;
                    }
                }
                false
            });

            let Some(nat_filtering) = station.options.iter().find_map(|v| match v {
                NatFiltering(v) => Some(v),
                _ => None
            }) else {
                return Err(Core_Exception);
            };

            let Some(nat_mapping) = station.options.iter().find_map(|v| match v {
                NatMapping(v) => Some(v),
                _ => None
            }) else {
                return Err(Core_Exception);
            };

            if !is_public || (*nat_filtering == 0 && *nat_mapping == 0){
                private_station = Some(station.clone());
            }

            if is_public{
                public_station = Some(station);
            }
        }

        let Some(mut private_station) = private_station else {
            return Err(Core_Exception);
        };

        let mut public_station = if let Some(public_station) = public_station{
            public_station
        } else {
            let mut public_station = private_station.clone();

            public_station.options.retain(|v| {
                match v {
                    Address(_) | Port(_) | NatFiltering(_) | NatMapping(_) | NatType(_) => false,
                    _ => true
                }
            });

            public_station.options.push(Address(*self.ip.regular_socket_addr.ip()));
            public_station.options.push(Port(self.ip.regular_socket_addr.port()));
            public_station.options.push(NatFiltering(0));
            public_station.options.push(NatMapping(0));
            public_station.options.push(NatType(3));

            public_station
        };

        let mut both = [&mut public_station, &mut private_station];

        for station in both{
            station.options.retain(|v| {
                match v {
                    PrincipalID(_) | RVConnectionID(_) => false,
                    _ => true
                }
            });

            station.options.push(PrincipalID(self.pid));
            station.options.push(RVConnectionID(cid));
        }


        let mut lock = self.station_url.write().await;
        *lock = vec![
            public_station.clone(),
            private_station
        ];
        drop(lock);

        let result = QResult::success(ErrorCode::Core_Unknown);

        let out = public_station.to_string();

        println!("out: {}", out);

        Ok((result, cid, public_station))
    }

    async fn replace_url(&self, target_url: StationUrl, dest: StationUrl) -> Result<(), ErrorCode> {
        let mut lock = self.station_url.write().await;

        let Some(target_addr) = target_url.options.iter().find(|v| matches!(v, Address(_))) else{
            return Err(ErrorCode::Core_InvalidArgument);
        };

        let Some(target_port) = target_url.options.iter().find(|v| matches!(v, Port(_))) else{
            return Err(ErrorCode::Core_InvalidArgument);
        };

        let Some(replacement_target) = lock.iter_mut().find(|url| {
            url.options.iter().any(|o| o == target_addr) &&
                url.options.iter().any(|o| o == target_port)
        }) else {
            return Err(ErrorCode::Core_InvalidArgument);
        };
        *replacement_target = dest;

        drop(lock);

        Ok(())
    }
}

impl MatchmakeExtension for User {
    async fn get_playing_session(&self, pids: Vec<u32>) -> Result<Vec<()>, ErrorCode> {
        Ok(Vec::new())
    }

    async fn update_progress_score(&self, gid: u32, progress: u8) -> Result<(), ErrorCode> {
        let mut sessions = self.matchmake_manager.sessions.read().await;

        let Some(session) = sessions.get(&gid) else {
            return Err(RendezVous_SessionVoid);
        };

        let session = session.clone();
        drop(sessions);

        let mut session = session.lock().await;

        session.session.progress_score = progress;

        Ok(())
    }

    async fn create_matchmake_session_with_param(
        &self,
        session: CreateMatchmakeSessionParam,
    ) -> Result<MatchmakeSession, ErrorCode> {
        println!("{:?}", session);

        let gid = self.matchmake_manager.next_gid();

        let mut new_session = ExtendedMatchmakeSession::from_matchmake_session(
            gid,
            session.matchmake_session,
            &self.this.clone(),
        )
        .await;

        new_session.session.participation_count = session.participation_count as u32;
        new_session
            .add_player(self.this.clone(), session.join_message)
            .await;

        let session = new_session.session.clone();

        let mut sessions = self.matchmake_manager.sessions.write().await;
        sessions.insert(gid, Arc::new(Mutex::new(new_session)));
        drop(sessions);

        Ok(session)
    }

    async fn join_matchmake_session_with_param(
        &self,
        join_session_param: JoinMatchmakeSessionParam,
    ) -> Result<MatchmakeSession, ErrorCode> {
        let mut sessions = self.matchmake_manager.sessions.read().await;

        let Some(session) = sessions.get(&join_session_param.gid) else {
            return Err(ErrorCode::RendezVous_SessionVoid);
        };

        let session = session.clone();
        drop(sessions);

        let mut session = session.lock().await;

        session.connected_players.retain(|v| v.upgrade().is_some_and(|v| v.pid != self.pid));

        session
            .add_player(self.this.clone(), join_session_param.join_message)
            .await;

        let mm_session = session.session.clone();

        Ok(mm_session)
    }

    async fn auto_matchmake_with_param_postpone(&self, session: AutoMatchmakeParam) -> Result<MatchmakeSession, ErrorCode> {
        println!("{:?}", session.search_criteria);

        let AutoMatchmakeParam{
            join_message,
            participation_count,
            gid_for_participation_check,
            matchmake_session,
            additional_participants,
            ..
        } = session;

        self.create_matchmake_session_with_param(CreateMatchmakeSessionParam{
            join_message,
            participation_count,
            gid_for_participation_check,
            create_matchmake_session_option: 0,
            matchmake_session,
            additional_participants
        }).await
    }
}

impl Matchmake for User {
    async fn unregister_gathering(&self, gid: u32) -> Result<bool, ErrorCode> {
        Ok(true)
    }
    async fn get_session_urls(&self, gid: u32) -> Result<Vec<StationUrl>, ErrorCode> {
        let sessions = self.matchmake_manager.sessions.read().await;

        let Some(session) = sessions.get(&gid) else {
            return Err(ErrorCode::RendezVous_SessionVoid);
        };

        let session = session.clone();

        drop(sessions);

        let session = session.lock().await;

        let urls: Vec<_> =
            session
                .connected_players
                .iter()
                .filter_map(|v| v.upgrade())
                .filter(|u| u.pid == session.session.gathering.host_pid)
                .map(|u| async move {
                    u.station_url.read().await.clone()
                })
                .next()
                .ok_or(ErrorCode::RendezVous_SessionClosed)?
                .await;


        println!("{:?}", urls);

        Ok(urls)
    }
}

impl NatTraversal for User {
    async fn report_nat_properties(
        &self,
        nat_mapping: u32,
        nat_filtering: u32,
        _rtt: u32,
    ) -> Result<(), ErrorCode> {

        let mut urls = self.station_url.write().await;

        for station_url in urls.iter_mut() {
            station_url.options.retain(|o| match o {
                NatMapping(_) | NatFiltering(_) => false,
                _ => true
            });

            station_url.options.push(NatMapping(nat_mapping as u8));
            station_url.options.push(NatFiltering(nat_filtering as u8));
        }

        Ok(())
    }

    async fn request_probe_initiation(&self, station_to_probe: String) -> Result<(), ErrorCode> {
        info!("NO!");
        Err(RendezVous_AccountExpired)
    }

    async fn request_probe_initialization_ext(&self, target_list: Vec<String>, station_to_probe: String) -> Result<(), ErrorCode> {
        let users = self.matchmake_manager.users.read().await;

        println!("requesting station probe for {:?} to {:?}", target_list, station_to_probe);

        for target in target_list{
            let Ok(url) = StationUrl::try_from(target.as_ref()) else{
                continue;
            };

            let Some(RVConnectionID(v)) = url.options.into_iter().find(|o| { matches!(o, &RVConnectionID(_)) }) else{
                continue;
            };

            let Some(v) = users.get(&v) else{
                continue;
            };

            let Some(user) = v.upgrade() else {
                continue;
            };

            if let Err(e) = user.remote.request_probe_initiation(station_to_probe.clone()).await{
               error!("error whilest probing");
            }
        }

        info!("finished probing");

        Ok(())
    }
}
