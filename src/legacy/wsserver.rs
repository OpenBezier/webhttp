#[allow(unused_imports)]
use crate::{
    api_init_none_func,
    command::{ServerCommand, WakerManager},
    start,
    websocket::ROOM,
    ActorMsg, ServiceCallback, WsData,
};
use webproto::{decode_message, Message, MockMessage};

use actix_web::web::{scope, ServiceConfig};
use async_trait::async_trait;
use dashmap::DashMap;
use sea_orm::DatabaseConnection;
use std::{any::Any, sync::Arc};
use tracing::*;

type TargetMessage = MockMessage;

#[derive(Clone)]
pub struct DataHandler<
    T: Send + Sync + serde::de::DeserializeOwned + 'static + serde::ser::Serialize,
> {
    pub queue: Arc<DashMap<String, Option<T>>>,
    pub wake_mgt: WakerManager,
    pub database: Option<DatabaseConnection>,
}

impl<T: Send + Sync + serde::de::DeserializeOwned + 'static + serde::ser::Serialize>
    DataHandler<T>
{
    pub fn new(database: Option<DatabaseConnection>) -> DataHandler<T> {
        let handler = DataHandler {
            queue: Arc::new(DashMap::<String, Option<T>>::new()),
            wake_mgt: WakerManager::default(),
            database: database,
        };
        handler
    }
}

// #[async_trait]
impl<T: Send + Sync + serde::de::DeserializeOwned + 'static + serde::ser::Serialize> ServiceCallback
    for DataHandler<T>
{
    fn api_init(&self, _web_app: &mut ServiceConfig) {}

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn wsdata(
        &self,
        data: WsData,
        _consumer: Arc<dyn ServiceCallback>,
    ) -> anyhow::Result<ActorMsg> {
        match &data {
            WsData::WsMessage { data } => {
                let msg = decode_message::<T>(&data.data).unwrap();
                match msg {
                    Message::ClientCommand(command) => {
                        info!("recv client command request");
                        let event_id = command.event_id.clone();
                        let resp_msg = TargetMessage::mock_response();
                        let _resp_cmd =
                            ServerCommand::<T>::send_answer(&data.addr, event_id, &resp_msg).await;
                    }
                    Message::ServerCommand(command) => {
                        info!("recv server command response");
                        let event_id = command.event_id.clone();
                        // 不包含时说明已经超时了
                        self.queue
                            .entry(event_id)
                            .and_modify(|v| *v = Some(command.command));
                    }
                    Message::Indication(_indication) => {}
                }
            }
            _ => {}
        }
        anyhow::Ok(ActorMsg::Ok)
    }
}

pub fn start_wsserver(name: String) {
    let app_name = name.clone();
    let sys = actix_rt::System::with_tokio_rt(|| tokio::runtime::Runtime::new().unwrap());
    sys.block_on(async move {
        let handler = Arc::new(DataHandler::<TargetMessage>::new(None));
        handler.wake_mgt.start(1);

        tokio::spawn(command_thread(handler.clone()));

        let status = start(
            name.clone(),
            9000,
            None,
            Some(handler.clone()),
            Some("/ws/api".into()),
            Some(3),
            api_init_none_func,
        )
        .await;
        // let status = start(
        //     name.clone(),
        //     9000,
        //     None,
        //     None,
        //     None,
        //     Some(3),
        //     api_init,
        // )
        // .await;

        if status.is_err() {
            println!("start service with error: {:?}", status.err());
        }
    });
    println!("server {} has been exited", app_name);
    std::process::exit(0);
}

pub async fn command_thread(handler: Arc<DataHandler<TargetMessage>>) {
    let mock_data = TargetMessage::mock_request();
    loop {
        for each in ROOM.sessions.iter() {
            let addr = &each.value().1;
            let resp_cmd = super::command::ServerCommand::<TargetMessage>::send_command(
                addr,
                &mock_data,
                1,
                &handler.queue,
                &handler.wake_mgt,
            )
            .await;
            info!("recved client msg: {:?}", resp_cmd);
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

use actix_web::HttpResponse;
use actix_web::{get, Responder};
use time::macros::offset;
#[get("/health")]
pub async fn health() -> impl Responder {
    // info!("[health] is called");
    let now = time::OffsetDateTime::now_utc().to_offset(offset!(+8));
    let return_info = format!("running: {:?}", now,);
    HttpResponse::Ok().body(return_info)
}

pub fn api_init(web_app: &mut ServiceConfig) {
    println!("api init");
    web_app.service(scope("/api").service(health));
}
