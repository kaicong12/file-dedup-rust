mod database;
mod handlers;
mod services;

use actix_cors::Cors;
use actix_web::{App, HttpServer, middleware::Logger, web};
use database::{create_connection_pool, init_database};
use dotenv::dotenv;
use env_logger;
use handlers::auth::{login, register_user};
use handlers::files::{complete_upload, initiate_upload};
use handlers::health::health_check;

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    println!("Starting HTTP server at http://localhost:8080");

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // Create database connection pool
    let pool = create_connection_pool()
        .await
        .expect("Failed to create database pool");

    // Initialize database (create tables if they don't exist)
    init_database(&pool)
        .await
        .expect("Failed to initialize database");

    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allowed_origin("http://localhost:3000")
                    .allow_any_method()
                    .allow_any_header()
                    .supports_credentials(),
            )
            .app_data(web::Data::new(pool.clone()))
            .service(health_check)
            .service(login)
            .service(register_user)
            .service(initiate_upload)
            .service(complete_upload)
            // enable logger - always register Actix Web Logger middleware last
            .wrap(Logger::default())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
