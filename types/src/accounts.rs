use actix_web::{
    error::ResponseError,
    get,
    http::{header::ContentType, StatusCode},
    post, put,
    web::Data,
    web::Json,
    web::Path,
    HttpResponse,
};
use derive_more::Display;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct AccountCreationRequest {
    pub game_player_ids: Vec<u64>,
    pub big_blind: u8,
    pub small_blind: u8,
    pub buy_in: u8,
}

#[derive(Deserialize, Serialize)]
pub struct AccountCreationResponse {
    pub game_id: u64,
}

#[derive(Deserialize, Serialize)]
pub struct PlayerAccountCreationRequest {
    pub username: String,
}

#[derive(Deserialize, Serialize)]
pub struct PlayerAccountCreationResponse {
    pub account_id: u64,
}

#[derive(Debug, Display)]
pub enum AccountCreationError {
    AccountCreationFailed,
    BadTaskRequest,
}

impl ResponseError for AccountCreationError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match self {
            AccountCreationError::AccountCreationFailed => StatusCode::FAILED_DEPENDENCY,
            AccountCreationError::BadTaskRequest => StatusCode::BAD_REQUEST,
        }
    }
}