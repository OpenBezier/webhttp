use super::{super::AppState, wsconn::WsConn};
use actix_web::{get, web, HttpRequest, HttpResponse, Responder, Scope};
use actix_web_actors::ws;
use time::macros::offset;
use tracing::{debug, warn};

#[get("/health")]
pub async fn ws_health(_data: web::Data<AppState>) -> impl Responder {
    debug!("health is called");
    let now = time::OffsetDateTime::now_utc().to_offset(offset!(+8));
    let return_info = format!("running: {:?}", now,);
    HttpResponse::Ok().body(return_info)
}

/// connid can be uuid, actor is role
/// connid can be in multi client, if by this way, they are same group, the use actor to differ them
#[get("/{business}/{actor}/{connid}")]
pub async fn ws_entry(
    req: HttpRequest,
    stream: web::Payload,
    params: web::Path<(String, String, String)>,
    appdata: web::Data<AppState>,
) -> HttpResponse {
    // info!("req: {:?} params: {:?}", req, params);
    let (business, actor, connid) = params.into_inner();
    let ip = req.peer_addr().unwrap().to_string();

    let headers = req.headers();
    let token = if headers.contains_key("token") {
        headers.get("token").unwrap().to_str().unwrap().to_string()
    } else {
        warn!("@@@@@ token is null for this websocket connection @@@@@");
        "".to_string()
    };

    let wsconn = WsConn::new(
        ip,
        business,
        connid,
        actor,
        token,
        appdata.get_ref().clone(),
    );
    // let resp = ws::start(wsconn, &req, stream);
    let resp = ws::WsResponseBuilder::new(wsconn, &req, stream)
        .frame_size(1024 * 1024 * 64)
        .start();

    debug!("{:?}", resp);
    if let Ok(resp) = resp {
        resp
    } else {
        HttpResponse::BadRequest()
            .body(format!("start ws conn with error: {}", resp.err().unwrap()))
    }
}

pub fn ws_api() -> Scope {
    web::scope("").service(ws_entry).service(ws_health)
}
