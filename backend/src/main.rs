mod config;
mod database;
mod handlers;
mod metrics;
mod middleware;
mod observability;
mod services;
mod worker;

use actix_cors::Cors;
use actix_web::{App, HttpServer, middleware::Logger, web};
use sqlx::PgPool;
use std::sync::Arc;

use env_logger;
use handlers::auth::{login, register_user};
use handlers::files::{complete_upload, generate_presigned_url, initiate_upload};
use handlers::health::{health_check, metrics_test};
use metrics::{BusinessMetrics, DeduplicationMetrics};
use middleware::Auth;
use observability::init_observability;
use worker::spawn_worker_process;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Starting HTTP server at http://localhost:8080");

    // Initialize observability (tracing and metrics)
    init_observability()?;

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let env_variables = config::Config::initialize("../.env");

    // Create database connection pool
    let database_url = &env_variables.database_url;
    let pool = PgPool::connect(database_url)
        .await
        .expect("Failed to create database pool");

    sqlx::migrate!("./src/migrations").run(&pool).await?;

    // Initialize metrics
    let dedup_metrics = Arc::new(DeduplicationMetrics::new());
    let business_metrics = Arc::new(BusinessMetrics::new());

    log::info!("📊 Metrics system initialized");

    // Start the worker process
    spawn_worker_process(
        pool.clone(),
        env_variables.redis_url.clone(),
        env_variables.opensearch_url.clone(),
        env_variables.aws_profile_name.clone(),
        env_variables.bedrock_model_id.clone(),
    )
    .await?;

    log::info!("Worker process started");

    let dedup_metrics_clone = dedup_metrics.clone();
    dedup_metrics.record_file_processed("image", 233);
    let business_metrics_clone = business_metrics.clone();

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
            .app_data(web::Data::new(dedup_metrics_clone.clone()))
            .app_data(web::Data::new(business_metrics_clone.clone()))
            .service(health_check)
            .service(metrics_test)
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
