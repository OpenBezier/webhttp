use super::super::AppState;
use actix::prelude;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Clone, Debug, Deserialize, Serialize)]
pub enum ActorError {
    #[error("room already existed {0}")]
    RoomAlreadyExisted(String),
    #[error("room is not existed {0}")]
    RoomNotExisted(String),
    #[error("decode wot data with error")]
    DataDecodeError,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ActorMsg {
    Ok,
    ConnectError { info: String },
}

#[derive(prelude::Message, Default, Debug, Deserialize, Serialize, Clone)]
#[rtype(result = "()")]
pub struct OutMessage {
    pub data: Vec<u8>,
}

// #[derive(Clone, Debug, Deserialize, Serialize)]
// pub enum MessageType {
//     Binary,
//     Text,
// }

#[derive(prelude::Message, Clone)]
#[rtype(result = "anyhow::Result<ActorMsg>")]
pub struct InMessage {
    pub addr: prelude::Recipient<OutMessage>,
    pub conn: ConnInfo,
    pub state: AppState,
    pub data: Vec<u8>,
}

// ----------------------- Connect and Disconnect -----------------------
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ConnInfo {
    pub ip: String,
    pub business: String,
    pub connid: String,
    pub actor: String,
    pub token: String,
}

#[derive(prelude::Message, Clone)]
#[rtype(result = "anyhow::Result<ActorMsg>")]
pub struct Connect {
    pub addr: prelude::Recipient<OutMessage>,
    pub conn: ConnInfo,
    pub state: AppState,
}

#[derive(prelude::Message, Clone)]
#[rtype(result = "anyhow::Result<ActorMsg>")]
pub struct Disconnect {
    pub conn: ConnInfo,
    pub state: AppState,
}

impl ConnInfo {
    pub fn get_session_id(&self) -> String {
        format!("{}_{}", self.actor, self.connid)
    }

    pub fn get_room_id(&self) -> String {
        self.connid.clone()
    }
}
