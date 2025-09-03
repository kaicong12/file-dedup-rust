use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    Resource,
    metrics::{MeterProvider, PeriodicReader, SdkMeterProvider},
    runtime,
    trace::{RandomIdGenerator, Sampler, Tracer},
};
use prometheus::{Encoder, TextEncoder};
use std::time::Duration;
use tracing::{info, warn};
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_observability() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing
    init_tracing()?;

    // Initialize metrics
    init_metrics()?;

    info!("Observability initialized successfully");
    Ok(())
}

fn init_tracing() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://otel-collector:4317".to_string());

    // Create OTLP tracer
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(&otlp_endpoint),
        )
        .with_trace_config(
            opentelemetry_sdk::trace::config()
                .with_sampler(Sampler::AlwaysOn)
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(Resource::new(vec![
                    KeyValue::new("service.name", "file-dedup-backend"),
                    KeyValue::new("service.version", "0.1.0"),
                    KeyValue::new("service.namespace", "file-dedup"),
                ])),
        )
        .install_batch(runtime::Tokio)?;

    // Create tracing layer
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Initialize subscriber with multiple layers
    Registry::default()
        .with(EnvFilter::from_default_env().add_directive("backend=debug".parse()?))
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .json(),
        )
        .with(telemetry_layer)
        .init();

    Ok(())
}

fn init_metrics() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://otel-collector:4317".to_string());

    // Create OTLP metrics exporter
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(&otlp_endpoint);

    let reader = PeriodicReader::builder(
        opentelemetry_otlp::new_pipeline()
            .metrics(runtime::Tokio)
            .with_exporter(exporter)
            .build()?,
        runtime::Tokio,
    )
    .with_interval(Duration::from_secs(5))
    .build();

    let meter_provider = SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(Resource::new(vec![
            KeyValue::new("service.name", "file-dedup-backend"),
            KeyValue::new("service.version", "0.1.0"),
            KeyValue::new("service.namespace", "file-dedup"),
        ]))
        .build();

    global::set_meter_provider(meter_provider);

    Ok(())
}

pub fn create_prometheus_metrics_handler() -> impl Fn() -> String {
    move || {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        match encoder.encode_to_string(&metric_families) {
            Ok(metrics) => metrics,
            Err(e) => {
                warn!("Failed to encode metrics: {}", e);
                String::new()
            }
        }
    }
}

// Custom metrics for the file deduplication service
pub struct FileDeduplicationMetrics {
    pub files_processed_total: prometheus::Counter,
    pub duplicates_found_total: prometheus::Counter,
    pub deduplication_duration_seconds: prometheus::Histogram,
    pub active_jobs: prometheus::Gauge,
    pub storage_bytes_saved: prometheus::Counter,
}

impl FileDeduplicationMetrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        let files_processed_total = prometheus::Counter::new(
            "files_processed_total",
            "Total number of files processed for deduplication",
        )?;

        let duplicates_found_total = prometheus::Counter::new(
            "duplicates_found_total",
            "Total number of duplicate files found",
        )?;

        let deduplication_duration_seconds = prometheus::Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "deduplication_duration_seconds",
                "Time spent processing files for deduplication",
            )
            .buckets(vec![0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0]),
        )?;

        let active_jobs = prometheus::Gauge::new(
            "active_deduplication_jobs",
            "Number of active deduplication jobs",
        )?;

        let storage_bytes_saved = prometheus::Counter::new(
            "storage_bytes_saved_total",
            "Total bytes saved through deduplication",
        )?;

        // Register metrics
        prometheus::register(Box::new(files_processed_total.clone()))?;
        prometheus::register(Box::new(duplicates_found_total.clone()))?;
        prometheus::register(Box::new(deduplication_duration_seconds.clone()))?;
        prometheus::register(Box::new(active_jobs.clone()))?;
        prometheus::register(Box::new(storage_bytes_saved.clone()))?;

        Ok(Self {
            files_processed_total,
            duplicates_found_total,
            deduplication_duration_seconds,
            active_jobs,
            storage_bytes_saved,
        })
    }
}

impl Default for FileDeduplicationMetrics {
    fn default() -> Self {
        Self::new().expect("Failed to create metrics")
    }
}

// HTTP metrics middleware
use actix_web::{
    Error, HttpMessage,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
};
use futures_util::future::LocalBoxFuture;
use prometheus::{Counter, Histogram, HistogramVec, IntCounterVec};
use std::{
    future::{Ready, ready},
    rc::Rc,
    time::Instant,
};

pub struct PrometheusMetrics {
    pub http_requests_total: IntCounterVec,
    pub http_request_duration_seconds: HistogramVec,
}

impl PrometheusMetrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        let http_requests_total = IntCounterVec::new(
            prometheus::Opts::new("http_requests_total", "Total HTTP requests"),
            &["method", "status", "endpoint"],
        )?;

        let http_request_duration_seconds = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP request duration in seconds",
            )
            .buckets(vec![
                0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]),
            &["method", "status", "endpoint"],
        )?;

        prometheus::register(Box::new(http_requests_total.clone()))?;
        prometheus::register(Box::new(http_request_duration_seconds.clone()))?;

        Ok(Self {
            http_requests_total,
            http_request_duration_seconds,
        })
    }
}

pub struct PrometheusMetricsMiddleware {
    metrics: Rc<PrometheusMetrics>,
}

impl PrometheusMetricsMiddleware {
    pub fn new(metrics: Rc<PrometheusMetrics>) -> Self {
        Self { metrics }
    }
}

impl<S, B> Transform<S, ServiceRequest> for PrometheusMetricsMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = PrometheusMetricsMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(PrometheusMetricsMiddlewareService {
            service,
            metrics: self.metrics.clone(),
        }))
    }
}

pub struct PrometheusMetricsMiddlewareService<S> {
    service: S,
    metrics: Rc<PrometheusMetrics>,
}

impl<S, B> Service<ServiceRequest> for PrometheusMetricsMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start_time = Instant::now();
        let method = req.method().to_string();
        let path = req.path().to_string();
        let metrics = self.metrics.clone();

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;
            let duration = start_time.elapsed().as_secs_f64();
            let status = res.status().as_u16().to_string();

            // Record metrics
            metrics
                .http_requests_total
                .with_label_values(&[&method, &status, &path])
                .inc();

            metrics
                .http_request_duration_seconds
                .with_label_values(&[&method, &status, &path])
                .observe(duration);

            Ok(res)
        })
    }
}
