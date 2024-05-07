mod api;
use api::{
    accounts::{ create_aze_game_account, create_aze_player_account },
    action::aze_poker_game_action
};
use actix_web::{ HttpServer, App, middleware::Logger };

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // std::env::set_var("RUST_LOG", "debug");
    // std::env::set_var("RUST_BACKTRACE", "1");
    // env_logger::init();

    HttpServer::new(move || {
        let _ = Logger::default();
        App::new()
            .service(create_aze_game_account)
            .service(create_aze_player_account)
            .service(aze_poker_game_action)
    })
        .bind(("127.0.0.1", 8000))?
        .run().await
}
