mod database;
mod handlers;
mod services;

use actix_web::{App, HttpServer, middleware::Logger, web};
use database::{create_connection_pool, init_database};
use dotenv::dotenv;
use env_logger;
use handlers::auth::auth_handlers::login;
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
            .app_data(web::Data::new(pool.clone()))
            .service(health_check)
            .service(login)
            // enable logger - always register Actix Web Logger middleware last
            .wrap(Logger::default())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
