use super::super::AppState;
use super::msg::{ActorMsg, ConnInfo, Connect, Disconnect, InMessage, OutMessage};
use tracing::{error, trace, warn};

use actix::{fut, ActorContext, ActorFutureExt, ContextFutureSpawner, WrapFuture};
use actix::{Actor, Running, StreamHandler};
use actix::{AsyncContext, Handler};
use actix_web_actors::ws;
use actix_web_actors::ws::Message::Text;
use rand::seq::SliceRandom;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(20);

pub struct WsConn {
    pub hb: Instant,
    pub ip: String,       // client IP
    pub business: String, // business name
    pub connid: String,   // client ID, can be roome name
    pub actor: String,    // role name
    pub token: String,    // token info
    pub state: AppState,

    pub in_room: Arc<Mutex<bool>>,
    pub exit_lock: Arc<Mutex<Option<Vec<u8>>>>,
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
            exit_lock: Arc::new(Mutex::new(None)),
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
        // start heartbeat
        self.hb(ctx);

        ctx.set_mailbox_capacity(1000);
        let addr = ctx.address();

        // send connect info to one random actor worker
        let in_room_copied = self.in_room.clone();
        let worker = self
            .state
            .worker
            .as_ref()
            .unwrap()
            .choose(&mut rand::thread_rng())
            .unwrap();
        worker
            .send(Connect {
                addr: addr.recipient(),
                conn: self.get_conn_info(),
                state: self.state.clone(),
            })
            .into_actor(self)
            .then(move |res, _conn, ctx| {
                match res {
                    Ok(_res) => match _res {
                        Ok(_msg) => match _msg {
                            ActorMsg::Ok => {
                                *in_room_copied.lock().unwrap() = true;
                            }
                            _ => {}
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
        // send disconnect info to one random actor worker
        if *self.in_room.lock().unwrap() {
            let worker = self
                .state
                .worker
                .as_ref()
                .unwrap()
                .choose(&mut rand::thread_rng())
                .unwrap();
            worker.do_send(Disconnect {
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
            Ok(ws::Message::Binary(bin)) => {
                // info!("binary msg");
                // send message info to one random actor worker
                let worker = self
                    .state
                    .worker
                    .as_ref()
                    .unwrap()
                    .choose(&mut rand::thread_rng())
                    .unwrap();

                // use sync type and not get the result
                // let status = worker.try_send(InMessage {
                //     addr: ctx.address().recipient(),
                //     conn: self.get_conn_info(),
                //     state: self.state.clone(),
                //     data: bin.to_vec(),
                // });
                // if status.is_err() {
                //     warn!("send msg to worker err: {:?}", status.err());
                // }

                // use async type and get result
                worker
                    .send(InMessage {
                        addr: ctx.address().recipient(),
                        conn: self.get_conn_info(),
                        state: self.state.clone(),
                        data: bin.to_vec(),
                    })
                    .into_actor(self)
                    .then(move |res, _, _ctx| {
                        match res {
                            Ok(_res) => match _res {
                                Ok(_msg) => match _msg {
                                    ActorMsg::Ok => {}
                                    ActorMsg::ConnectError { info } => {
                                        error!("send msg to worker err: {:?}", info);
                                    }
                                },
                                Err(_e) => {
                                    error!("send msg with error: {:?}", _e);
                                }
                            },
                            _ => {
                                error!("send msg with error: actor call error");
                            }
                        }
                        fut::ready(())
                    })
                    .wait(ctx);
            }
            Ok(Text(_s)) => {
                // let worker = self.state.worker.choose(&mut rand::thread_rng()).unwrap();
                // worker.do_send(InMessage {
                //     addr: ctx.address().recipient(),
                //     conn: self.get_conn_info(),
                //     state: self.state.clone(),
                //     data: s.as_bytes().to_vec(),
                // });
                let warn_text = "BinaryOnly".as_bytes();
                warn!("recved text content, should only use binary format");
                ctx.binary(warn_text);
                ctx.stop();
            }
            Err(e) => {
                // panic!("{}", e);
                error!("websocket msg error: {:?}", e);
            }
        }
    }
}

impl Handler<OutMessage> for WsConn {
    type Result = ();
    fn handle(&mut self, msg: OutMessage, ctx: &mut Self::Context) {
        ctx.binary(msg.data);
    }
}
