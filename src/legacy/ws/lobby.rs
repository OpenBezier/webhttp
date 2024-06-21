use super::super::{ServiceCallback, WsData};
use super::msg::{ActorMsg, ConnInfo, Connect, Disconnect, Request, Response};
use super::msg_bin::BinRecvMessage;

use actix::prelude::{Actor, Context, Handler, Recipient};
use dashmap::{DashMap, DashSet};
use std::sync::Arc;
use tracing::{error, info};

type Socket = Recipient<Response>;

#[derive(Clone)]
pub struct Lobby {
    // appstate: Arc<Mutex<Option<super::super::AppState>>>,
    pub consumer: Arc<dyn ServiceCallback>,
    pub sessions: Arc<DashMap<String, (ConnInfo, Socket)>>, // key is connid_actor
    pub rooms: Arc<DashMap<String, DashSet<String>>>, // key is room connid name, and value is connid_actor list
}

impl Actor for Lobby {
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        let mailbox_size = 2048;
        ctx.set_mailbox_capacity(mailbox_size);
        info!("set lobby mailbox size as {}", mailbox_size);
    }
}

impl Lobby {
    pub fn new(consumer: Arc<dyn ServiceCallback>) -> Lobby {
        let lobby = Lobby {
            consumer: consumer,
            sessions: Arc::new(DashMap::new()),
            rooms: Arc::new(DashMap::new()),
        };
        lobby
    }

    pub fn send_response(&self, message: Response, id_to: &String) {
        if let Some(socket_recipient) = self.sessions.get(id_to) {
            let _ = socket_recipient.1.do_send(message);
        } else {
            error!(
                "attempting to send message but couldn't find user id: {}",
                id_to
            );
        }
    }

    pub fn send_group_json(&self, connid: String, evt_data: String) {
        self.rooms
            .get(&connid)
            .unwrap()
            .iter()
            .for_each(|connid_actor| {
                let connid_actor = connid_actor.clone();
                let mut resp = Response::default();
                resp.status = true;
                resp.text = Some(evt_data.clone());
                self.send_response(resp, &connid_actor);
            });
    }

    pub fn send_group_binary(&self, connid: String, evt_data: Vec<u8>) {
        self.rooms
            .get(&connid)
            .unwrap()
            .iter()
            .for_each(|connid_actor| {
                let connid_actor = connid_actor.clone();
                let mut resp = Response::default();
                resp.status = true;
                resp.binary = Some(evt_data.clone());
                self.send_response(resp, &connid_actor);
            });
    }
}

impl Handler<Connect> for Lobby {
    type Result = anyhow::Result<ActorMsg>;
    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        // call consumer
        futures::executor::block_on(async {
            self.consumer
                .wsdata(WsData::WsConnect { data: msg }, self.consumer.clone())
                .await
        })
    }
}

impl Handler<Disconnect> for Lobby {
    type Result = anyhow::Result<ActorMsg>;
    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) -> Self::Result {
        // call consumer
        futures::executor::block_on(async {
            self.consumer
                .wsdata(WsData::WsDisconnect { data: msg }, self.consumer.clone())
                .await
        })
    }
}

impl Handler<Request> for Lobby {
    type Result = anyhow::Result<ActorMsg>;
    fn handle(&mut self, msg: Request, _ctx: &mut Context<Self>) -> Self::Result {
        // call consumer
        futures::executor::block_on(async {
            self.consumer
                .wsdata(WsData::WsRequest { data: msg }, self.consumer.clone())
                .await
        })
    }
}

impl Handler<BinRecvMessage> for Lobby {
    type Result = anyhow::Result<ActorMsg>;
    fn handle(&mut self, msg: BinRecvMessage, _ctx: &mut Context<Self>) -> Self::Result {
        futures::executor::block_on(async {
            self.consumer
                .wsdata(WsData::WsBinMsg { data: msg }, self.consumer.clone())
                .await
        })
    }
}
