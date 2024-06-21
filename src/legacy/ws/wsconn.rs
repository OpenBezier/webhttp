use super::super::AppState;
use super::msg::{ActorMsg, ConnInfo, Connect, Disconnect, MsgType, Request, Response};
use super::msg_bin::{BinRecvMessage, BinSendMessage};
use tracing::*;

use actix::{fut, ActorContext, ActorFutureExt, ContextFutureSpawner, WrapFuture};
use actix::{Actor, Running, StreamHandler};
use actix::{AsyncContext, Handler};
use actix_web_actors::ws;
use actix_web_actors::ws::Message::Text;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{trace, warn};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(20);

pub struct WsConn {
    pub hb: Instant,
    pub ip: String,       // client IP
    pub business: String, // 业务类型
    pub connid: String,   // 连接Client的连接ID，可用于Room的名字，相同的应该在一个Room
    pub actor: String,    // 角色名称
    pub token: String,    // 用户令牌
    pub state: AppState,

    pub in_room: Arc<Mutex<bool>>,
}

impl WsConn {
    pub fn new(
        ip: String,
        business: String,
        connid: String,
        actor: String,
        token: String,
        state: AppState,
    ) -> WsConn {
        WsConn {
            hb: Instant::now(),
            ip: ip,
            business: business,
            connid: connid,
            actor: actor,
            token: token,
            state: state,
            in_room: Arc::new(Mutex::new(false)),
        }
    }

    fn get_conn_info(&self) -> ConnInfo {
        ConnInfo {
            ip: self.ip.clone(),
            business: self.business.clone(),
            connid: self.connid.clone(),
            actor: self.actor.clone(),
            token: self.token.clone(),
        }
    }
}

impl Actor for WsConn {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // 启动心跳线程
        self.hb(ctx);

        ctx.set_mailbox_capacity(100);
        let addr = ctx.address();

        // 发送连接建立的Connect信息到Lobby
        let in_room_copied = self.in_room.clone();
        self.state
            .lobby
            .send(Connect {
                addr: addr.recipient(),
                conn: self.get_conn_info(),
                state: self.state.clone(),
            })
            .into_actor(self)
            .then(move |res, _, ctx| {
                match res {
                    Ok(_res) => match _res {
                        Ok(_msg) => match _msg {
                            ActorMsg::Ok => {
                                *in_room_copied.lock().unwrap() = true;
                            }
                            ActorMsg::ConnectError { info } => {
                                error!("add to room with error: {:?}", info);
                                *in_room_copied.lock().unwrap() = false;
                                ctx.stop();
                            }
                        },
                        Err(_e) => {
                            error!("add to room with error: {:?}", _e);
                            *in_room_copied.lock().unwrap() = false;
                            ctx.stop();
                        }
                    },
                    _ => {
                        error!("add to room with erro: actor call error");
                        *in_room_copied.lock().unwrap() = false;
                        ctx.stop();
                    }
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // 发送断开连接的DisConnect信息到Lobby
        if *self.in_room.lock().unwrap() {
            self.state.lobby.do_send(Disconnect {
                conn: self.get_conn_info(),
                state: self.state.clone(),
            });
        }
        Running::Stop
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {}
}

impl WsConn {
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                warn!("Disconnecting failed heartbeat");
                ctx.stop();
                return;
            }
            trace!("sending ping info");
            ctx.ping(b"ping");
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsConn {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Ok(ws::Message::Continuation(_)) => {
                ctx.stop();
            }
            Ok(ws::Message::Nop) => (),
            Ok(ws::Message::Binary(bin)) => self.state.lobby.do_send(BinRecvMessage {
                addr: ctx.address().recipient(),
                conn: self.get_conn_info(),
                state: self.state.clone(),
                data: bin.to_vec(),
            }),
            Ok(Text(s)) => self.state.lobby.do_send(Request {
                addr: ctx.address().recipient(),
                conn: self.get_conn_info(),
                state: self.state.clone(),
                data: MsgType::MsgText {
                    data: s.to_string(),
                },
            }),
            Err(e) => panic!("{}", e),
        }
    }
}

impl Handler<Response> for WsConn {
    type Result = ();
    fn handle(&mut self, msg: Response, ctx: &mut Self::Context) {
        trace!("resp data len: {}", msg.info.len());
        ctx.text(json!(msg).to_string());

        // status为false时断开此websocket连接
        if !msg.status {
            ctx.stop();
        }
    }
}

impl Handler<BinSendMessage> for WsConn {
    type Result = ();
    fn handle(&mut self, msg: BinSendMessage, ctx: &mut Self::Context) {
        ctx.binary(msg.data);
    }
}
