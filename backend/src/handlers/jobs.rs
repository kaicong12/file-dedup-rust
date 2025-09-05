use actix_web::{HttpResponse, Responder, delete, get, web};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct Job {
    pub job_id: Uuid,
    pub file_id: i32,
    pub file_name: String,
    pub file_path: Option<String>,
    pub s3_key: String,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Deserialize)]
pub struct JobsQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[get("/jobs")]
pub async fn get_jobs(query: web::Query<JobsQuery>, db_pool: web::Data<PgPool>) -> impl Responder {
    let limit = query.limit.unwrap_or(50).min(100); // Max 100 jobs per request
    let offset = query.offset.unwrap_or(0);

    let result = if let Some(ref status) = query.status {
        sqlx::query(
            "SELECT job_id, file_id, file_name, file_path, s3_key, status, error_message, created_at, updated_at, completed_at 
             FROM jobs WHERE status = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        )
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(db_pool.get_ref())
        .await
    } else {
        sqlx::query(
            "SELECT job_id, file_id, file_name, file_path, s3_key, status, error_message, created_at, updated_at, completed_at 
             FROM jobs ORDER BY created_at DESC LIMIT $1 OFFSET $2"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(db_pool.get_ref())
        .await
    };

    match result {
        Ok(rows) => {
            let jobs: Vec<Job> = rows
                .into_iter()
                .map(|row| Job {
                    job_id: row.get("job_id"),
                    file_id: row.get("file_id"),
                    file_name: row.get("file_name"),
                    file_path: row.get("file_path"),
                    s3_key: row.get("s3_key"),
                    status: row.get("status"),
                    error_message: row.get("error_message"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                    completed_at: row.get("completed_at"),
                })
                .collect();

            HttpResponse::Ok().json(serde_json::json!({
                "jobs": jobs,
                "total": jobs.len(),
                "limit": limit,
                "offset": offset
            }))
        }
        Err(e) => {
            log::error!("Failed to fetch jobs: {}", e);
            HttpResponse::InternalServerError().json("Failed to fetch jobs")
        }
    }
}

#[get("/jobs/{job_id}")]
pub async fn get_job_by_id(path: web::Path<Uuid>, db_pool: web::Data<PgPool>) -> impl Responder {
    let job_id = path.into_inner();

    println!("Job id: {job_id:?}");

    match sqlx::query(
        "SELECT job_id, file_id, file_name, file_path, s3_key, status, error_message, created_at, updated_at, completed_at 
         FROM jobs WHERE job_id = $1"
    )
    .bind(job_id)
    .fetch_optional(db_pool.get_ref())
    .await
    {
        Ok(Some(row)) => {
            let job = Job {
                job_id: row.get("job_id"),
                file_id: row.get("file_id"),
                file_name: row.get("file_name"),
                file_path: row.get("file_path"),
                s3_key: row.get("s3_key"),
                status: row.get("status"),
                error_message: row.get("error_message"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                completed_at: row.get("completed_at"),
            };

            HttpResponse::Ok().json(job)
        }
        Ok(None) => HttpResponse::NotFound().json("Job not found"),
        Err(e) => {
            log::error!("Failed to fetch job {}: {}", job_id, e);
            HttpResponse::InternalServerError().json("Failed to fetch job")
        }
    }
}

#[delete("/jobs/{job_id}")]
pub async fn delete_job(path: web::Path<Uuid>, db_pool: web::Data<PgPool>) -> impl Responder {
    let job_id = path.into_inner();

    // First check if the job exists
    match sqlx::query("SELECT job_id FROM jobs WHERE job_id = $1")
        .bind(job_id)
        .fetch_optional(db_pool.get_ref())
        .await
    {
        Ok(Some(_)) => {
            // Job exists, proceed with deletion
            match sqlx::query("DELETE FROM jobs WHERE job_id = $1")
                .bind(job_id)
                .execute(db_pool.get_ref())
                .await
            {
                Ok(result) => {
                    if result.rows_affected() > 0 {
                        log::info!("Successfully deleted job {}", job_id);
                        HttpResponse::Ok().json(serde_json::json!({
                            "message": "Job deleted successfully",
                            "job_id": job_id
                        }))
                    } else {
                        HttpResponse::InternalServerError().json("Failed to delete job")
                    }
                }
                Err(e) => {
                    log::error!("Failed to delete job {}: {}", job_id, e);
                    HttpResponse::InternalServerError().json("Failed to delete job")
                }
            }
        }
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Job not found",
            "job_id": job_id
        })),
        Err(e) => {
            log::error!("Failed to check job existence {}: {}", job_id, e);
            HttpResponse::InternalServerError().json("Failed to check job")
        }
    }
}

/// Create a new job record in the database
pub async fn create_job_record(
    db_pool: &PgPool,
    job_id: Uuid,
    file_id: i32,
    file_name: &str,
    file_path: Option<&str>,
    s3_key: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO jobs (job_id, file_id, file_name, file_path, s3_key, status) 
         VALUES ($1, $2, $3, $4, $5, 'pending')",
    )
    .bind(job_id)
    .bind(file_id)
    .bind(file_name)
    .bind(file_path)
    .bind(s3_key)
    .execute(db_pool)
    .await?;

    Ok(())
}

/// Update job status in the database
pub async fn update_job_status_in_db(
    db_pool: &PgPool,
    job_id: Uuid,
    status: &str,
    error_message: Option<&str>,
) -> Result<(), sqlx::Error> {
    let completed_at = if status == "completed" || status == "failed" {
        Some(chrono::Utc::now())
    } else {
        None
    };

    sqlx::query(
        "UPDATE jobs SET status = $1, error_message = $2, updated_at = NOW(), completed_at = $3 
         WHERE job_id = $4",
    )
    .bind(status)
    .bind(error_message)
    .bind(completed_at)
    .bind(job_id)
    .execute(db_pool)
    .await?;

    Ok(())
}
