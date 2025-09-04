use log::info;
use opentelemetry::global;
use opentelemetry_otlp::{Protocol, WithExportConfig};
use opentelemetry_sdk::{Resource, metrics::SdkMeterProvider};

pub fn init_observability() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing
    // init_tracing()?;

    // Initialize metrics
    init_metrics()?;

    info!("Observability initialized successfully");
    Ok(())
}

// fn init_tracing() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//     let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
//         .unwrap_or_else(|_| "http://otel-collector:4317".to_string());

//     // Create OTLP tracer
//     let tracer = opentelemetry_otlp::new_pipeline()
//         .tracing()
//         .with_exporter(
//             opentelemetry_otlp::new_exporter()
//                 .tonic()
//                 .with_endpoint(&otlp_endpoint),
//         )
//         .with_trace_config(
//             opentelemetry_sdk::trace::config()
//                 .with_sampler(Sampler::AlwaysOn)
//                 .with_id_generator(RandomIdGenerator::default())
//                 .with_resource(Resource::new(vec![
//                     KeyValue::new("service.name", "file-dedup-backend"),
//                     KeyValue::new("service.version", "0.1.0"),
//                     KeyValue::new("service.namespace", "file-dedup"),
//                 ])),
//         )
//         .install_batch(runtime::Tokio)?;

//     // Create tracing layer
//     let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

//     // Initialize subscriber with multiple layers
//     Registry::default()
//         .with(EnvFilter::from_default_env().add_directive("backend=debug".parse()?))
//         .with(
//             tracing_subscriber::fmt::layer()
//                 .with_target(false)
//                 .with_thread_ids(true)
//                 .with_file(true)
//                 .with_line_number(true)
//                 .json(),
//         )
//         .with(telemetry_layer)
//         .init();

//     Ok(())
// }

fn init_metrics() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://otel-collector:4318".to_string());

    // Create OTLP metrics exporter
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpBinary)
        .with_endpoint(&otlp_endpoint)
        .build()?;

    let meter_provider = SdkMeterProvider::builder()
        .with_periodic_exporter(exporter)
        .with_resource(
            Resource::builder()
                .with_service_name("file-dedup-backend")
                .build(),
        )
        .build();

    global::set_meter_provider(meter_provider);

    Ok(())
}
