mod config;
mod database;
mod handlers;
mod middleware;
mod services;
mod worker;

use actix_cors::Cors;
use actix_web::{App, HttpServer, middleware::Logger, web};
use sqlx::PgPool;

use env_logger;
use handlers::auth::{login, register_user};
use handlers::files::{complete_upload, generate_presigned_url, initiate_upload};
use handlers::health::health_check;
use middleware::Auth;
use worker::spawn_worker_process;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting HTTP server at http://localhost:8080");

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let env_variables = config::Config::initialize("../.env");

    // Create database connection pool
    let database_url = &env_variables.database_url;
    let pool = PgPool::connect(database_url)
        .await
        .expect("Failed to create database pool");

    sqlx::migrate!("./src/migrations").run(&pool).await?;

    // Start the worker process
    let worker_handle = spawn_worker_process(
        pool.clone(),
        env_variables.redis_url.clone(),
        env_variables.opensearch_url.clone(),
        env_variables.aws_profile_name.clone(),
        env_variables.bedrock_model_id.clone(),
    )
    .await?;

    log::info!("Worker process started");

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
            .app_data(web::Data::new(env_variables.clone()))
            .service(health_check)
            .service(login)
            .service(register_user)
            .service(
                web::scope("")
                    .wrap(Auth::new(env_variables.jwt_secret.clone()))
                    .service(initiate_upload)
                    .service(complete_upload)
                    .service(generate_presigned_url),
            )
            // enable logger - always register Actix Web Logger middleware last
            .wrap(Logger::default())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
    .map_err(Into::into)
}
