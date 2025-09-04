use opentelemetry::metrics::{Counter, Gauge, Histogram};
use opentelemetry::{KeyValue, global};
use std::time::Instant;

/// OpenTelemetry-based metrics for the file deduplication service
/// These integrate directly with your existing OpenTelemetry setup
pub struct DeduplicationMetrics {
    // Business Metrics - counters that always go up
    pub files_processed_total: Counter<u64>,
    pub duplicates_found_total: Counter<u64>,
    pub clusters_created_total: Counter<u64>,
    pub storage_saved_bytes: Counter<u64>,

    // System Health Metrics - current state values
    pub active_jobs: Gauge<i64>,
    pub queue_size: Gauge<i64>,
    pub failed_jobs_total: Counter<u64>,
    pub opensearch_errors_total: Counter<u64>,
    pub s3_errors_total: Counter<u64>,

    // Performance Metrics - histograms for timing
    pub deduplication_duration: Histogram<f64>,
    pub embedding_generation_duration: Histogram<f64>,
    pub opensearch_query_duration: Histogram<f64>,
    pub s3_operation_duration: Histogram<f64>,

    // File type breakdown - counter with labels
    pub files_by_type: Counter<u64>,
    pub similarity_scores: Histogram<f64>,
}

impl DeduplicationMetrics {
    pub fn new() -> Self {
        let meter = global::meter("file-dedup-backend");

        Self {
            // Business Metrics
            files_processed_total: meter
                .u64_counter("files_processed_total")
                .with_description("Total number of files processed for deduplication")
                .build(),

            duplicates_found_total: meter
                .u64_counter("duplicates_found_total")
                .with_description("Total number of duplicate files found")
                .build(),

            clusters_created_total: meter
                .u64_counter("clusters_created_total")
                .with_description("Total number of file clusters created")
                .build(),

            storage_saved_bytes: meter
                .u64_counter("storage_saved_bytes")
                .with_description("Total bytes of storage saved through deduplication")
                .build(),

            // System Health Metrics
            active_jobs: meter
                .i64_gauge("active_jobs")
                .with_description("Number of currently active deduplication jobs")
                .build(),

            queue_size: meter
                .i64_gauge("queue_size")
                .with_description("Number of jobs waiting in the queue")
                .build(),

            failed_jobs_total: meter
                .u64_counter("failed_jobs_total")
                .with_description("Total number of failed deduplication jobs")
                .build(),

            opensearch_errors_total: meter
                .u64_counter("opensearch_errors_total")
                .with_description("Total number of OpenSearch errors")
                .build(),

            s3_errors_total: meter
                .u64_counter("s3_errors_total")
                .with_description("Total number of S3 errors")
                .build(),

            // Performance Metrics
            deduplication_duration: meter
                .f64_histogram("deduplication_duration_seconds")
                .with_description("Time taken to complete deduplication process")
                .build(),

            embedding_generation_duration: meter
                .f64_histogram("embedding_generation_duration_seconds")
                .with_description("Time taken to generate file embeddings")
                .build(),

            opensearch_query_duration: meter
                .f64_histogram("opensearch_query_duration_seconds")
                .with_description("Time taken for OpenSearch queries")
                .build(),

            s3_operation_duration: meter
                .f64_histogram("s3_operation_duration_seconds")
                .with_description("Time taken for S3 operations")
                .build(),

            // File Type Metrics
            files_by_type: meter
                .u64_counter("files_by_type_total")
                .with_description("Total files processed by file type")
                .build(),

            similarity_scores: meter
                .f64_histogram("similarity_scores")
                .with_description("Distribution of similarity scores between files")
                .build(),
        }
    }

    /// Record a file being processed - increment counter with file type label
    pub fn record_file_processed(&self, file_type: &str, file_size: u64) {
        self.files_processed_total.add(1, &[]);
        self.files_by_type
            .add(1, &[KeyValue::new("file_type", file_type.to_string())]);

        log::info!(
            "📊 File processed: type={}, size={} bytes",
            file_type,
            file_size
        );
    }

    /// Record duplicates found - increment counters
    pub fn record_duplicates_found(&self, count: u64, storage_saved: u64) {
        self.duplicates_found_total.add(count, &[]);
        self.storage_saved_bytes.add(storage_saved, &[]);

        log::info!(
            "🔍 Found {} duplicates, saved {} bytes",
            count,
            storage_saved
        );
    }

    /// Record cluster creation - increment counter
    pub fn record_cluster_created(&self) {
        self.clusters_created_total.add(1, &[]);
        log::info!("🗂️ Cluster created");
    }

    /// Record similarity score - add to histogram
    pub fn record_similarity_score(&self, score: f64) {
        self.similarity_scores.record(score, &[]);
        log::debug!("📈 Similarity score: {:.3}", score);
    }

    /// Record job failure - increment with error type label
    pub fn record_job_failure(&self, error_type: &str) {
        self.failed_jobs_total
            .add(1, &[KeyValue::new("error_type", error_type.to_string())]);
        log::warn!("❌ Job failed: {}", error_type);
    }

    /// Record OpenSearch error
    pub fn record_opensearch_error(&self, operation: &str) {
        self.opensearch_errors_total
            .add(1, &[KeyValue::new("operation", operation.to_string())]);
        log::error!("🔍❌ OpenSearch error in {}", operation);
    }

    /// Record S3 error
    pub fn record_s3_error(&self, operation: &str) {
        self.s3_errors_total
            .add(1, &[KeyValue::new("operation", operation.to_string())]);
        log::error!("☁️❌ S3 error in {}", operation);
    }

    /// Update queue metrics - set current values
    pub fn update_queue_metrics(&self, active_jobs: i64, queue_size: i64) {
        self.active_jobs.record(active_jobs, &[]);
        self.queue_size.record(queue_size, &[]);
        log::debug!("📋 Queue: {} active, {} queued", active_jobs, queue_size);
    }

    /// Record processing duration
    pub fn record_deduplication_duration(&self, duration_seconds: f64, job_type: &str) {
        self.deduplication_duration.record(
            duration_seconds,
            &[KeyValue::new("job_type", job_type.to_string())],
        );
        log::info!("⏱️ Deduplication completed in {:.3}s", duration_seconds);
    }

    /// Record embedding generation duration
    pub fn record_embedding_duration(&self, duration_seconds: f64, file_type: &str) {
        self.embedding_generation_duration.record(
            duration_seconds,
            &[KeyValue::new("file_type", file_type.to_string())],
        );
        log::debug!("🧠 Embedding generated in {:.3}s", duration_seconds);
    }

    /// Record OpenSearch query duration
    pub fn record_opensearch_duration(&self, duration_seconds: f64, operation: &str) {
        self.opensearch_query_duration.record(
            duration_seconds,
            &[KeyValue::new("operation", operation.to_string())],
        );
        log::debug!("🔍 OpenSearch {} in {:.3}s", operation, duration_seconds);
    }

    /// Record S3 operation duration
    pub fn record_s3_duration(&self, duration_seconds: f64, operation: &str) {
        self.s3_operation_duration.record(
            duration_seconds,
            &[KeyValue::new("operation", operation.to_string())],
        );
        log::debug!("☁️ S3 {} in {:.3}s", operation, duration_seconds);
    }
}

/// Business-level metrics using OpenTelemetry
pub struct BusinessMetrics {
    pub deduplication_ratio: Gauge<f64>,
    pub average_cluster_size: Gauge<f64>,
    pub processing_throughput: Gauge<f64>,
    pub storage_efficiency: Gauge<f64>,
    pub cost_savings: Gauge<f64>,
}

impl BusinessMetrics {
    pub fn new() -> Self {
        let meter = global::meter("file-dedup-business");

        Self {
            deduplication_ratio: meter
                .f64_gauge("deduplication_ratio")
                .with_description("Ratio of duplicate files to total files processed")
                .build(),

            average_cluster_size: meter
                .f64_gauge("average_cluster_size")
                .with_description("Average number of files per cluster")
                .build(),

            processing_throughput: meter
                .f64_gauge("processing_throughput_files_per_minute")
                .with_description("Number of files processed per minute")
                .build(),

            storage_efficiency: meter
                .f64_gauge("storage_efficiency")
                .with_description("Percentage of storage saved through deduplication")
                .build(),

            cost_savings: meter
                .f64_gauge("cost_savings_dollars_per_month")
                .with_description("Estimated monthly cost savings in dollars")
                .build(),
        }
    }

    pub fn update_deduplication_ratio(&self, duplicates: u64, total_files: u64) {
        if total_files > 0 {
            let ratio = (duplicates as f64 / total_files as f64) * 100.0;
            self.deduplication_ratio.record(ratio, &[]);
            log::info!("📊 Deduplication ratio: {:.2}%", ratio);
        }
    }

    pub fn update_average_cluster_size(&self, total_files_in_clusters: u64, cluster_count: u64) {
        if cluster_count > 0 {
            let avg_size = total_files_in_clusters as f64 / cluster_count as f64;
            self.average_cluster_size.record(avg_size, &[]);
            log::info!("🗂️ Average cluster size: {:.2} files", avg_size);
        }
    }

    pub fn update_throughput(&self, files_processed: u64, time_window_minutes: f64) {
        if time_window_minutes > 0.0 {
            let throughput = files_processed as f64 / time_window_minutes;
            self.processing_throughput.record(throughput, &[]);
            log::info!("⚡ Throughput: {:.2} files/min", throughput);
        }
    }

    pub fn calculate_cost_savings(&self, storage_saved_gb: f64, cost_per_gb_per_month: f64) {
        let savings = storage_saved_gb * cost_per_gb_per_month;
        self.cost_savings.record(savings, &[]);
        log::info!("💰 Cost savings: ${:.2}/month", savings);
    }

    pub fn update_storage_efficiency(&self, storage_saved: u64, total_storage: u64) {
        if total_storage > 0 {
            let efficiency = (storage_saved as f64 / total_storage as f64) * 100.0;
            self.storage_efficiency.record(efficiency, &[]);
            log::info!("📈 Storage efficiency: {:.2}%", efficiency);
        }
    }
}

/// Simple timer for measuring operations with OpenTelemetry
pub struct MetricsTimer {
    start_time: Instant,
    operation_name: String,
}

impl MetricsTimer {
    pub fn new(operation_name: String) -> Self {
        log::debug!("⏱️ Starting: {}", operation_name);
        Self {
            start_time: Instant::now(),
            operation_name,
        }
    }

    /// Finish timing and record to the appropriate histogram
    pub fn finish_deduplication(self, metrics: &DeduplicationMetrics, job_type: &str) -> f64 {
        let duration = self.start_time.elapsed().as_secs_f64();
        metrics.record_deduplication_duration(duration, job_type);
        duration
    }

    pub fn finish_embedding(self, metrics: &DeduplicationMetrics, file_type: &str) -> f64 {
        let duration = self.start_time.elapsed().as_secs_f64();
        metrics.record_embedding_duration(duration, file_type);
        duration
    }

    pub fn finish_opensearch(self, metrics: &DeduplicationMetrics, operation: &str) -> f64 {
        let duration = self.start_time.elapsed().as_secs_f64();
        metrics.record_opensearch_duration(duration, operation);
        duration
    }

    pub fn finish_s3(self, metrics: &DeduplicationMetrics, operation: &str) -> f64 {
        let duration = self.start_time.elapsed().as_secs_f64();
        metrics.record_s3_duration(duration, operation);
        duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        // This will use the global OpenTelemetry meter
        let metrics = DeduplicationMetrics::new();
        let business_metrics = BusinessMetrics::new();

        // Test that we can call the methods without panicking
        metrics.record_file_processed("image", 1024);
        metrics.record_duplicates_found(5, 5120);
        metrics.record_cluster_created();

        business_metrics.update_deduplication_ratio(10, 100);
        business_metrics.update_average_cluster_size(50, 10);
        business_metrics.update_throughput(120, 2.0);
    }

    #[test]
    fn test_timer() {
        let metrics = DeduplicationMetrics::new();
        let timer = MetricsTimer::new("test_operation".to_string());

        std::thread::sleep(std::time::Duration::from_millis(10));
        let duration = timer.finish_deduplication(&metrics, "test");

        assert!(duration >= 0.01); // At least 10ms
        assert!(duration < 1.0); // Less than 1 second
    }
}
