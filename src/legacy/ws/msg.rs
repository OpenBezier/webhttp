use super::super::AppState;
use super::msg_bin::BinSendMessage;
use actix::prelude::{Message, Recipient};
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

// ----------------------- Connect and Disconnect -----------------------
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ConnInfo {
    pub ip: String,
    pub business: String,
    pub connid: String,
    pub actor: String,
    pub token: String,
}

#[derive(Message, Clone)]
#[rtype(result = "anyhow::Result<ActorMsg>")]
pub struct Connect {
    pub addr: Recipient<Response>,
    pub conn: ConnInfo,
    pub state: AppState,
}

#[derive(Message, Clone)]
// #[rtype(result = "()")]
#[rtype(result = "anyhow::Result<ActorMsg>")]
pub struct Disconnect {
    pub conn: ConnInfo,
    pub state: AppState,
}

// ----------------------- Response -----------------------
#[derive(Message, Default, Debug, Deserialize, Serialize, Clone)]
#[rtype(result = "()")]
pub struct Response {
    pub status: bool,
    pub info: String,
    pub binary: Option<Vec<u8>>,
    pub text: Option<String>,
}

// ----------------------- Request -----------------------
#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum MsgType {
    MsgText { data: String },
    MsgBinary { data: Vec<u8> },
}

#[derive(Message, Clone)]
// #[rtype(result = "()")]
#[rtype(result = "anyhow::Result<ActorMsg>")]
pub struct Request {
    pub addr: Recipient<Response>,
    pub conn: ConnInfo,
    pub state: AppState,
    pub data: MsgType,
}
