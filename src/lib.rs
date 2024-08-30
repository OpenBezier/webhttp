pub mod command;
pub mod response;
pub mod websocket;
pub use response::{NoneBodyData, Response};
pub mod access_token;
pub use access_token::{get_token_and_path, AccessToken, TokenPermission};
pub mod termenv;
pub use termenv::{termenv_check, termenv_get, termenv_init};
pub mod permission;
pub use permission::*;

pub mod mysql;
pub mod redis;
pub mod webhttp;

use actix::Actor;
use actix::Addr;
use actix_settings::ApplySettings;
use actix_web::web::PayloadConfig;
use crossbeam::queue::SegQueue;
use websocket::{ActorMsg, Connect, Disconnect, InMessage};

use actix_cors::Cors;
#[allow(unused_imports)]
use actix_web::{
    dev::{Server, Service},
    http::{KeepAlive, Method},
    middleware, web,
    web::ServiceConfig,
    App, HttpResponse, HttpServer, *,
};

// use actix_web::middleware::Logger;
use env_logger::Env;
use sea_orm::DatabaseConnection;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use time::macros::offset;
#[allow(unused_imports)]
use tracing::*;

#[allow(clippy::unused_async)]
// #[cfg_attr(coverage, no_coverage)]
pub async fn not_found() -> HttpResponse {
    HttpResponse::NotFound().body("Not Found 404")
}

#[allow(clippy::unused_async)]
// #[cfg_attr(coverage, no_coverage)]
pub async fn health_check() -> HttpResponse {
    let now = time::OffsetDateTime::now_utc().to_offset(offset!(+8));
    let return_info = format!("running: {:?}", now,);
    HttpResponse::Ok().body(return_info)
}

pub enum WsData {
    WsMessage { data: InMessage },
    WsConnect { data: Connect },
    WsDisconnect { data: Disconnect },
}

pub fn api_init_none_func(_: &mut ServiceConfig) {}

// public trait that outsite code should use
// use async_trait::async_trait;
// #[async_trait(?Send)]
// #[async_trait]
pub trait ServiceCallback: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn api_init(&self, web_app: &mut ServiceConfig);
    fn wsdata(&self, data: WsData, consumer: Arc<dyn ServiceCallback>) -> anyhow::Result<ActorMsg>;
}

#[derive(Clone)]
pub struct AppState {
    pub worker: Option<Vec<Addr<websocket::Worker>>>,
    pub consumer: Option<Arc<dyn ServiceCallback>>,
    pub database: Option<DatabaseConnection>,
    pub redis: Option<fred::prelude::RedisPool>,
    pub config: Option<serde_json::Value>,
    pub wsapi: Option<String>,
    pub token_check: Option<Arc<dyn crate::access_token::TokenPermission + Send + Sync>>,
    pub jwt_secret: Option<String>,
}

pub async fn start(
    name: String,                                                       // server name
    port: u16,                                                          // server port
    config: Option<serde_json::Value>,                                  // configuration
    ws_consumer: Option<Arc<dyn ServiceCallback>>,                      // Websocket consumer
    ws_api: Option<String>,                                             // Websocket api register
    worker_num: Option<usize>,                                          // actor number of websocket
    api_init: impl Fn(&mut web::ServiceConfig) + Sync + Send + 'static, // outer api register
    thread_num: Option<usize>,                                          // web thread number
    database: Option<DatabaseConnection>,                               // database connector
    redis: Option<fred::prelude::RedisPool>,                            // redis connector
    token_check: Option<Arc<dyn crate::access_token::TokenPermission + Send + Sync>>, // permissison
    jwt_secret: Option<String>,                                         // jwt secret
    api_prefix: Option<String>, // api prefix url, such as /api/v1/test
) -> anyhow::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let new_addr_list = if ws_consumer.is_some() {
        let copied_consumer = ws_consumer.clone().unwrap();
        let worker_thread_number = if worker_num.is_none() {
            3
        } else {
            worker_num.unwrap()
        };
        let worker_addr = Arc::new(SegQueue::<Addr<websocket::Worker>>::default());

        // all arbiter actor number = worker_num * 2
        for _i in 0..worker_thread_number {
            let cusumer_copied = copied_consumer.clone();
            let worker_addr_copied = worker_addr.clone();
            let arbiter = actix_rt::Arbiter::new();
            arbiter.spawn(async move {
                let addr = websocket::Worker::new(cusumer_copied.clone()).start();
                worker_addr_copied.push(addr);
                let addr = websocket::Worker::new(cusumer_copied).start();
                worker_addr_copied.push(addr);
            });
        }

        // check arbiter actor number
        loop {
            if worker_addr.len() != worker_thread_number * 2 {
                continue;
            } else {
                break;
            }
        }

        let mut new_addr_list = Vec::<Addr<websocket::Worker>>::default();
        for _i in 0..worker_addr.len() {
            new_addr_list.push(worker_addr.pop().unwrap());
        }
        Some(new_addr_list)
    } else {
        None
    };

    let state = AppState {
        worker: new_addr_list,
        consumer: ws_consumer,
        database: database,
        redis: redis,
        config: config,
        wsapi: ws_api,
        token_check: token_check,
        jwt_secret: jwt_secret,
    };
    start_internal(state, name, port, api_init, thread_num, api_prefix).await?;
    anyhow::Ok(())
}

async fn start_internal(
    state: AppState,
    name: String,
    port: u16,
    api_init: impl Fn(&mut web::ServiceConfig) + Sync + Send + 'static,
    thread_num: Option<usize>,
    api_prefix: Option<String>,
) -> anyhow::Result<()> {
    let mut api_prefix = if api_prefix.is_some() {
        api_prefix.unwrap().clone()
    } else {
        "/".into()
    };
    if !api_prefix.ends_with("/") {
        api_prefix = format!("{}/", api_prefix);
    }

    let metrics = actix_web_prom::PrometheusMetricsBuilder::new(&name)
        .endpoint(format!("{}metrics", api_prefix).as_str())
        .build()
        .unwrap();
    let found_errors = prometheus::IntCounterVec::new(
        prometheus::Opts {
            namespace: name.clone(),
            subsystem: String::new(),
            name: "errors".into(),
            help: "FoundErrors".into(),
            const_labels: HashMap::new(),
            variable_labels: Vec::new(),
        },
        &["path", "description"],
    )?;
    metrics.registry.register(Box::new(found_errors.clone()))?;

    let api_init = Arc::new(api_init);
    let payload_config = PayloadConfig::new(16 * 1024 * 1024);
    let json_payload_config = web::JsonConfig::default();
    let json_payload_config = json_payload_config.limit(16 * 1024 * 1024);

    let mut settings = actix_settings::Settings::from_default_template();
    actix_settings::Settings::override_field(&mut settings.actix.mode, "production")?;
    actix_settings::Settings::override_field(
        &mut settings.actix.hosts,
        format!("[[\"0.0.0.0\", {}]]", port),
    )?;
    if thread_num.is_some() {
        let thread_str = thread_num.unwrap().to_string();
        actix_settings::Settings::override_field(&mut settings.actix.num_workers, thread_str)?;
    } else {
        actix_settings::Settings::override_field(&mut settings.actix.num_workers, "4")?;
    }

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .expose_any_header()
            .max_age(3600);
        let error_metrics = found_errors.clone();
        App::new()
            .app_data(payload_config.clone())
            .app_data(json_payload_config.clone())
            .app_data(web::Data::new(state.clone()))
            .route(
                format!("{}health", api_prefix).as_str(),
                web::get().to(health_check),
            )
            .configure(init_service(
                state.clone(),
                api_init.clone(),
                api_prefix.clone(),
            ))
            .wrap(cors)
            // not use middlewar of logger, because the format is not same as tracing
            // .wrap(Logger::new("%a %r[%t]-%s %T %b"))
            .wrap(metrics.clone())
            .wrap_fn(move |req, srv| {
                // let now = time::OffsetDateTime::now_utc().to_offset(offset!(+8));
                let start_time = std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_micros();
                // with port
                let addr = req.request().peer_addr().unwrap().to_string();
                // without port
                // let addr = req
                //     .request()
                //     .connection_info()
                //     .realip_remote_addr()
                //     .unwrap()
                //     .to_string();
                let method = req.request().method().to_string();
                let path = req.request().path().to_string();

                let fut = srv.call(req);
                let error_counter = error_metrics.clone();
                async move {
                    let srv_response = fut.await?;
                    if let Some(err) = srv_response.response().error() {
                        let url = match srv_response.request().match_pattern() {
                            Some(pattern) => pattern,
                            None => String::new(),
                        };
                        let err_desc = format!("{err}");
                        error_counter
                            .clone()
                            .with_label_values(&[url.as_str(), err_desc.as_str()])
                            .inc();
                    }

                    let end_time = std::time::SystemTime::now()
                        .duration_since(std::time::SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_micros();
                    let micros_diff = (end_time - start_time) as f32 / 1000.0;

                    // let log_info = format!(
                    //     "[{:?}] {} {} {} {} {}ms",
                    //     now,
                    //     addr,
                    //     method,
                    //     path,
                    //     srv_response.response().status().as_str(),
                    //     micros_diff
                    // );
                    let log_info = format!(
                        "{} {} {} {} {}ms",
                        addr,
                        method,
                        path,
                        srv_response.response().status().as_str(),
                        micros_diff
                    );
                    if path.eq("/metrics") {
                        debug!("{}", log_info)
                    } else {
                        info!("{}", log_info)
                    }

                    Ok(srv_response)
                }
            })
            .default_service(web::route().to(not_found))
    })
    // .keep_alive(std::time::Duration::from_secs(75))
    // .keep_alive(KeepAlive::Os)
    .apply_settings(&settings)
    // .bind(("0.0.0.0", port))? // if use settings, it already has binding addr, can't add again
    .run()
    .await?;
    Ok(())
}

fn init_service(
    state: AppState,
    api_init: Arc<impl Fn(&mut web::ServiceConfig) + Send + Sync>,
    api_prefix: String,
) -> impl Fn(&mut web::ServiceConfig) {
    move |web_app| {
        if state.consumer.is_some() {
            // info!("http and websocket mode");
            let mut ws_api_url =
                if state.wsapi.is_some() && !state.wsapi.as_ref().unwrap().is_empty() {
                    let ws_api_url = state.wsapi.as_ref().unwrap();
                    info!("register ws api service as: {}", ws_api_url);
                    ws_api_url
                } else {
                    let ws_api_url = "websocket/api";
                    info!("register ws api service as default: {}", ws_api_url);
                    ws_api_url
                };

            if ws_api_url.starts_with("/") {
                ws_api_url = ws_api_url.strip_prefix("/").unwrap();
            }

            web_app.service(
                web::scope(format!("{}{}", api_prefix, ws_api_url).as_str())
                    .service(websocket::api::ws_api()),
            );
            state.consumer.as_ref().unwrap().api_init(web_app);
            return;
        }
        // info!("http mode only");
        api_init(web_app);
    }
}
