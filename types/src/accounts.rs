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
pub struct AccountCreationResponse {
    pub is_created: bool,
}

#[derive(Deserialize, Serialize)]
pub struct PlayerAccountCreationResponse {
    pub is_created: bool,
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