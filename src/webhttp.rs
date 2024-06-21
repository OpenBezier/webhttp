use crate::start;
use actix_web::web::{scope, ServiceConfig};
use sea_orm::DatabaseConnection;
use tracing::error;

#[derive(Clone)]
pub struct AppState {
    pub database: DatabaseConnection,
    pub redis: fred::prelude::RedisPool,
}

fn api_init(web_app: &mut ServiceConfig) {
    web_app.service(scope("/api/v1/webhttp"));
}

pub fn server_main() -> anyhow::Result<()> {
    let name: String = "webhttp".into();
    let app_name = name.clone();
    let sys = actix_rt::System::with_tokio_rt(|| {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name(name.clone().as_str())
            .enable_all()
            .global_queue_interval(31)
            .build()
            .unwrap();
        runtime
    });

    sys.block_on(async move {
        let http_port: u16 = 5010;
        let status = start(
            name.clone(),
            http_port,
            None,
            None,
            Some(format!("/api/{}/websocket", name)),
            Some(3),
            api_init,
            Some(2),
            None,
            None,
            None,
            None,
        )
        .await;

        if status.is_err() {
            error!("start service with error: {:?}", status.err());
        }
    });

    println!("server {} has been exited", app_name);
    std::process::exit(0);
}
