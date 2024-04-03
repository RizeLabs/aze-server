use actix_web::{get, post, put, delete, Responder, HttpResponse};

#[post("/v1/game/deal")]
pub async fn deal() -> impl Responder {
    // fetch all the player id, participating in a particular game using account_get_item
    // for now hardcoding player ids

    
    HttpResponse::Ok().body("deal")
}