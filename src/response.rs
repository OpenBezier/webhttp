use actix_web::{http::StatusCode, HttpResponse};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Deserialize, Serialize, Debug)]
pub struct NoneBodyData {}

#[derive(Debug)]
pub struct Response<T: Serialize + Debug> {
    pub code: StatusCode,
    pub message: String,
    pub data: Option<T>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Status {
    pub status: bool,
    pub code: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientRsp {
    pub status: bool,
    pub code: u16,
    pub message: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OkData<T: Serialize + Debug> {
    pub status: bool,
    pub code: u16,
    pub message: T,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ErrData {
    pub status: bool,
    pub code: u16,
    pub message: String,
}

impl<T: Serialize + Debug> Response<T> {
    #[allow(dead_code)]
    pub fn success(data: T) -> Self {
        Self {
            code: StatusCode::OK,
            message: "".to_string(),
            data: Some(data),
        }
    }

    #[allow(dead_code)]
    pub fn internal_error(message: &str) -> Self {
        Self {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.to_string(),
            data: None,
        }
    }

    #[allow(dead_code)]
    pub fn bad_request(message: &str) -> Self {
        Self {
            code: StatusCode::BAD_REQUEST,
            message: message.to_string(),
            data: None,
        }
    }

    #[allow(dead_code)]
    pub fn forbidden(message: &str) -> Self {
        Self {
            code: StatusCode::FORBIDDEN,
            message: message.to_string(),
            data: None,
        }
    }

    #[allow(dead_code)]
    pub fn not_acceptable(message: &str) -> Self {
        Self {
            code: StatusCode::NOT_ACCEPTABLE,
            message: message.to_string(),
            data: None,
        }
    }

    #[allow(dead_code)]
    pub fn unauthorized(message: &str) -> Self {
        Self {
            code: StatusCode::UNAUTHORIZED,
            message: message.to_string(),
            data: None,
        }
    }

    #[allow(dead_code)]
    pub fn nofound(message: &str) -> Self {
        Self {
            code: StatusCode::NOT_FOUND,
            message: message.to_string(),
            data: None,
        }
    }

    pub fn finished(&self) -> HttpResponse {
        if self.code.is_success() {
            HttpResponse::Ok().json(serde_json::json!(OkData {
                status: true,
                code: self.code.as_u16(),
                message: self.data.as_ref().unwrap(),
            }))
        } else {
            HttpResponse::Ok().json(serde_json::json!(ErrData {
                status: false,
                code: self.code.as_u16(),
                message: self.message.clone(),
            }))
        }
    }
}

impl<T: Serialize + Debug> std::fmt::Display for Response<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ code: {}, message: {} }}", self.code, self.message)
    }
}

impl<T: Serialize + Debug> actix_web::error::ResponseError for Response<T> {
    fn status_code(&self) -> StatusCode {
        self.code.clone()
    }

    fn error_response(&self) -> HttpResponse {
        // HttpResponse::build(self.code).json(serde_json::json!(ErrData {
        //     status: false,
        //     code: self.code.as_u16(),
        //     message: self.message.clone(),
        // }))
        HttpResponse::Ok().json(serde_json::json!(ErrData {
            status: false,
            code: self.code.as_u16(),
            message: self.message.clone(),
        }))
    }
}
