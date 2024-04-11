mod api;
mod model;
// mod accounts;

use api::accounts::{create_aze_game_account, create_aze_player_account};
use api::notes::{deal};
use actix_web::{HttpServer, App, middleware::Logger};
use aze_lib::client::create_aze_client;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug");
    std::env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();

    HttpServer::new(move || {
        let _ = Logger::default();
        App::new()
            .service(create_aze_game_account)
            .service(create_aze_player_account)
            .service(deal)
    })
    .bind(("127.0.0.1", 8000))?
    .run()
    .await
}
