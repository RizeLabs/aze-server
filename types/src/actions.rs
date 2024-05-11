use actix_web::{
    error::ResponseError,
    get,
    http::{ header::ContentType, StatusCode },
    post,
    put,
    web::Data,
    web::Json,
    web::Path,
    HttpResponse,
};
use derive_more::Display;
use serde::{ Deserialize, Serialize };

#[derive(Debug, Display)]
pub enum GameActionError {
    GameActionFailed,
    BadActionRequest,
}

#[derive(Deserialize, Serialize)]
pub struct GameActionResponse {
    pub is_taken: bool,
}

impl ResponseError for GameActionError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match self {
            GameActionError::GameActionFailed => StatusCode::FAILED_DEPENDENCY,
            GameActionError::BadActionRequest => StatusCode::BAD_REQUEST,
        }
    }
}