use super::super::{ServiceCallback, WsData};
use super::msg::{ActorMsg, Connect, Disconnect, InMessage};
use actix::prelude::{Actor, Context, Handler};
use std::sync::Arc;
use tracing::info;

#[derive(Clone)]
pub struct Worker {
    pub consumer: Arc<dyn ServiceCallback>,
}

impl Actor for Worker {
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        let mailbox_size = 2048;
        ctx.set_mailbox_capacity(mailbox_size);
        info!("set worker mailbox size as {}", mailbox_size);
    }
}

impl Worker {
    pub fn new(consumer: Arc<dyn ServiceCallback>) -> Worker {
        Worker { consumer: consumer }
    }
}

impl Handler<Connect> for Worker {
    type Result = anyhow::Result<ActorMsg>;
    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        let status = self.consumer.wsdata(
            WsData::WsConnect { data: msg.clone() },
            self.consumer.clone(),
        );
        if status.is_ok() {
            let actor_msg = status.as_ref().clone().unwrap();
            match actor_msg {
                ActorMsg::Ok => {
                    super::ROOM.add(&msg)?;
                }
                _ => {}
            }
        }
        status
        // futures::executor::block_on(async {
        //     self.consumer
        //         .wsdata(WsData::WsConnect { data: msg }, self.consumer.clone())
        //         .await
        // })
    }
}

impl Handler<Disconnect> for Worker {
    type Result = anyhow::Result<ActorMsg>;
    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) -> Self::Result {
        super::ROOM.remove(&msg)?;
        self.consumer
            .wsdata(WsData::WsDisconnect { data: msg }, self.consumer.clone())
        // futures::executor::block_on(async {
        //     self.consumer
        //         .wsdata(WsData::WsDisconnect { data: msg }, self.consumer.clone())
        //         .await
        // })
    }
}

impl Handler<InMessage> for Worker {
    type Result = anyhow::Result<ActorMsg>;
    fn handle(&mut self, msg: InMessage, _ctx: &mut Context<Self>) -> Self::Result {
        self.consumer
            .wsdata(WsData::WsMessage { data: msg }, self.consumer.clone())
        // futures::executor::block_on(async {
        //     self.consumer
        //         .wsdata(WsData::WsMessage { data: msg }, self.consumer.clone())
        //         .await
        // })
    }
}
