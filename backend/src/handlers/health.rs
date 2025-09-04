use crate::metrics::{BusinessMetrics, DeduplicationMetrics};
use actix_web::{HttpResponse, Result, get, web};
use serde_json::json;
use std::sync::Arc;

#[get("/health")]
pub async fn health_check(
    dedup_metrics: web::Data<Arc<DeduplicationMetrics>>,
    business_metrics: web::Data<Arc<BusinessMetrics>>,
) -> Result<HttpResponse> {
    // Increment health check counter for testing
    dedup_metrics.files_processed_total.add(1, &[]);

    // Update some sample metrics for testing
    dedup_metrics.update_queue_metrics(3, 7);
    business_metrics.update_deduplication_ratio(25, 100);

    Ok(HttpResponse::Ok().json(json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": "file-dedup-rust",
        "metrics_enabled": true,
        "sample_data": {
            "files_processed": "incremented on each health check",
            "active_jobs": 3,
            "queue_size": 7,
            "deduplication_ratio": "25%"
        }
    })))
}

#[get("/metrics-test")]
pub async fn metrics_test(
    dedup_metrics: web::Data<Arc<DeduplicationMetrics>>,
    business_metrics: web::Data<Arc<BusinessMetrics>>,
) -> Result<HttpResponse> {
    // Generate sample metrics for testing
    dedup_metrics.record_file_processed("image", 2048);
    dedup_metrics.record_file_processed("document", 1024);
    dedup_metrics.record_duplicates_found(5, 10240);
    dedup_metrics.record_cluster_created();
    dedup_metrics.record_similarity_score(0.85);
    dedup_metrics.record_deduplication_duration(2.5, "batch");
    dedup_metrics.update_queue_metrics(2, 5);

    business_metrics.update_deduplication_ratio(15, 100);
    business_metrics.update_average_cluster_size(45, 10);
    business_metrics.update_throughput(120, 2.0);
    business_metrics.update_storage_efficiency(5120, 20480);

    Ok(HttpResponse::Ok().json(json!({
        "status": "metrics_generated",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "message": "Sample metrics have been generated for testing",
        "generated_metrics": {
            "files_processed": 2,
            "duplicates_found": 5,
            "clusters_created": 1,
            "storage_saved_bytes": 10240,
            "similarity_score": 0.85,
            "deduplication_duration": "2.5s",
            "active_jobs": 2,
            "queue_size": 5,
            "deduplication_ratio": "15%",
            "average_cluster_size": 4.5,
            "throughput": "60 files/min",
            "storage_efficiency": "25%"
        },
        "next_steps": [
            "Check OTel Collector at http://localhost:8889/metrics",
            "Check Prometheus targets at http://localhost:9090/targets",
            "Query metrics in Prometheus at http://localhost:9090"
        ]
    })))
}
