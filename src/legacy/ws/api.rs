use super::super::{wsconn::WsConn, AppState};
use actix_web::{get, web, HttpRequest, HttpResponse, Responder, Scope};
use actix_web_actors::ws;
use time::macros::offset;
use tracing::{debug, info, warn};

#[get("/health")]
pub async fn ws_health(_data: web::Data<AppState>) -> impl Responder {
    info!("/api/ws/health is called");
    let now = time::OffsetDateTime::now_utc().to_offset(offset!(+8));
    let return_info = format!("running: {:?}", now,);
    HttpResponse::Ok().body(return_info)
}

/// business是业务名称，connid为连接ID-通常为UUID，actor是角色名称
/// connid在多个client连接里可以是一样的，此时说明他们是一个Group，并且可以用actor来进行区分不同的角色
#[get("/{business}/{connid}/{actor}")]
pub async fn ws_entry(
    req: HttpRequest,
    stream: web::Payload,
    params: web::Path<(String, String, String)>,
    srv: web::Data<AppState>,
) -> HttpResponse {
    // info!("req: {:?} params: {:?}", req, params);
    let (business, connid, actor) = params.into_inner();
    let ip = req.peer_addr().unwrap().to_string();

    let headers = req.headers();
    let token = if headers.contains_key("token") {
        headers.get("token").unwrap().to_str().unwrap().to_string()
    } else {
        warn!("@@@@@ token is null for this websocket connection @@@@@");
        "".to_string()
    };

    let wsconn = WsConn::new(ip, business, connid, actor, token, srv.get_ref().clone());
    let resp = ws::start(wsconn, &req, stream);

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
