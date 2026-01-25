use actix_cors::Cors;
use actix_files::Files;
use actix_web::{middleware::Logger, web, App, HttpServer};
use dotenv::dotenv;
use std::sync::Arc;

mod config;
mod controllers;
mod db;
mod middleware;
mod models;

use config::Config;
use db::Database;

pub struct AppState {
    pub db: Arc<Database>,
    pub config: Config,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let config = Config::from_env();
    let port = config.port;

    log::info!("Initializing database at {}", config.database_url);
    let db = Database::new(&config.database_url).expect("Failed to initialize database");
    let db = Arc::new(db);

    log::info!("Starting Starfish Bot server on port {}", port);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .app_data(web::Data::new(AppState {
                db: Arc::clone(&db),
                config: config.clone(),
            }))
            .wrap(Logger::default())
            .wrap(cors)
            .configure(controllers::health::config)
            .configure(controllers::auth::config)
            .configure(controllers::dashboard::config)
            .service(Files::new("/", "./starfish-frontend").index_file("index.html"))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
