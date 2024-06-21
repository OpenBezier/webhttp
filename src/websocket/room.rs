use super::msg::{ActorMsg, ConnInfo, Connect, Disconnect, OutMessage};
use actix::prelude::Recipient;
use dashmap::{DashMap, DashSet};
use lazy_static::lazy_static;
use std::sync::Arc;
use tracing::info;

pub struct Room {
    /// key is actor_connid
    pub(crate) sessions: Arc<DashMap<String, (ConnInfo, Recipient<OutMessage>)>>,
    /// key is room connid name, and value is actor_connid list
    pub rooms: Arc<DashMap<String, DashSet<String>>>,
}

impl Room {
    pub fn new() -> Room {
        let lobby = Room {
            sessions: Arc::new(DashMap::new()),
            rooms: Arc::new(DashMap::new()),
            // mutexes: Arc::new(DashMap::new()),
        };
        lobby
    }

    pub fn get_client_id_list(&self) -> Vec<String> {
        let mut client_id_list = Vec::<String>::default();
        for each in self.sessions.iter() {
            client_id_list.push(each.key().clone());
        }
        client_id_list
    }

    pub fn get_client_conn_info_list(&self) -> Vec<ConnInfo> {
        let mut conn_info_list = Vec::<ConnInfo>::default();
        for each in self.sessions.iter() {
            conn_info_list.push(each.value().0.clone());
        }
        conn_info_list
    }

    pub fn has_client_conn(&self, connid: &String) -> bool {
        for each in self.sessions.iter() {
            if each.0.connid.eq(connid) {
                return true;
            }
        }
        false
    }

    pub fn get_client_id_by_conn(&self, connid: &String) -> Option<String> {
        for each in self.sessions.iter() {
            if each.0.connid.eq(connid) {
                return Some(each.0.get_session_id());
            }
        }
        None
    }

    pub fn get_client_conn_addr_list(&self) -> Vec<Recipient<OutMessage>> {
        let mut conn_addr_list = Vec::<Recipient<OutMessage>>::default();
        for each in self.sessions.iter() {
            conn_addr_list.push(each.value().1.clone());
        }
        conn_addr_list
    }

    pub fn add(&self, data: &Connect) -> anyhow::Result<ActorMsg> {
        // let id_to = format!("{}_{}", data.conn.actor, data.conn.connid);
        let id_to = data.conn.get_session_id();
        info!("ws connect info: {:?}", id_to);
        if self.sessions.contains_key(&id_to) {
            // if connid_actor aleady in sessions, return error
            return Err(anyhow::anyhow!(
                "already have this connid_actor name in sessions"
            ));
        }

        self.sessions.entry(id_to.clone()).or_insert_with(|| {
            let room_name = data.conn.get_room_id();
            self.rooms
                .entry(room_name.clone())
                .or_insert_with(DashSet::new)
                .insert(id_to.clone());
            // self.mutexes.entry(id_to.clone()).or_insert(false);
            (data.conn.clone(), data.addr.clone())
        });
        return anyhow::Ok(ActorMsg::Ok);
    }

    pub fn remove(&self, data: &Disconnect) -> anyhow::Result<ActorMsg> {
        // let id_to = format!("{}_{}", data.conn.actor, data.conn.connid);
        let id_to = data.conn.get_session_id();
        // if self.mutexes.remove(&id_to).is_some() {
        if self.sessions.remove(&id_to).is_some() {
            let mut delete_this_room = false;
            let room_name = data.conn.get_room_id();
            self.rooms.entry(room_name.clone()).and_modify(|e| {
                e.remove(&id_to);
                if e.len() == 0 {
                    delete_this_room = true;
                }
            });

            // if let Some(actor_list) = self.rooms.get_mut(&data.conn.connid) {
            //     actor_list.remove(&id_to);
            //     if actor_list.len() == 0 {
            //         delete_this_room = true;
            //     }
            // }
            if delete_this_room {
                info!("release room: {}", room_name);
                self.rooms.remove(&data.conn.connid);
            }
        }
        return anyhow::Ok(ActorMsg::Ok);
    }
}

lazy_static! {
    pub static ref ROOM: Room = Room::new();
}
