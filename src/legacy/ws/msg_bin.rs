use super::super::{AppState, ConnInfo};
use super::msg::ActorMsg;
use actix::prelude;
use serde::{Deserialize, Serialize};

#[derive(prelude::Message, Default, Debug, Deserialize, Serialize, Clone)]
#[rtype(result = "()")]
pub struct BinSendMessage {
    pub data: Vec<u8>,
}

#[derive(prelude::Message, Clone)]
#[rtype(result = "anyhow::Result<ActorMsg>")]
pub struct BinRecvMessage {
    pub addr: prelude::Recipient<BinSendMessage>,
    pub conn: ConnInfo,
    pub state: AppState,
    pub data: Vec<u8>,
}
