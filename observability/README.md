# Observability Stack for File Deduplication Service

This directory contains the complete observability setup using OpenTelemetry (OTel), Prometheus, Grafana, and Loki for comprehensive monitoring, metrics collection, and log aggregation.

## Architecture Overview

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Application   │───▶│  OTEL Collector  │───▶│   Prometheus    │
│  (Rust Backend) │    │                  │    │   (Metrics)     │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                       │                       │
         │                       ▼                       ▼
         │              ┌─────────────────┐    ┌─────────────────┐
         │              │      Loki       │    │     Grafana     │
         │              │     (Logs)      │    │ (Visualization) │
         │              └─────────────────┘    └─────────────────┘
         │                       ▲
         ▼                       │
┌─────────────────┐    ┌──────────────────┐
│    Promtail     │───▶│   Log Files &    │
│ (Log Collector) │    │ Docker Containers│
└─────────────────┘    └──────────────────┘
```

## Components

### 1. OpenTelemetry Collector

- **Purpose**: Receives, processes, and exports telemetry data (metrics, logs, traces)
- **Port**: 4317 (gRPC), 4318 (HTTP)
- **Configuration**: `otel/otel-collector-config.yml`

### 2. Prometheus

- **Purpose**: Time-series database for metrics storage
- **Port**: 9090
- **Configuration**: `prometheus/prometheus.yml`
- **Web UI**: http://localhost:9090

### 3. Grafana

- **Purpose**: Visualization and dashboarding
- **Port**: 3001 (to avoid conflict with Next.js on 3000)
- **Configuration**: `grafana/provisioning/`
- **Web UI**: http://localhost:3001
- **Default Login**: admin/admin

### 4. Loki

- **Purpose**: Log aggregation and storage
- **Port**: 3100
- **Configuration**: `loki/loki-config.yml`

### 5. Promtail

- **Purpose**: Log collection agent
- **Configuration**: `promtail/promtail-config.yml`
- **Collects**: Docker container logs, system logs, application logs

## Quick Start

### 1. Start the Observability Stack

```bash
# Start all services including observability
docker-compose up -d

# Or start only observability services
docker-compose up -d prometheus grafana loki promtail otel-collector
```

### 2. Access the Dashboards

- **Grafana**: http://localhost:3001 (admin/admin)
- **Prometheus**: http://localhost:9090
- **Application Metrics**: http://localhost:8080/metrics

### 3. View Pre-configured Dashboard

1. Open Grafana at http://localhost:3001
2. Login with admin/admin
3. Navigate to "Dashboards" → "File Deduplication Service Dashboard"

## Metrics Available

### Application Metrics

- `http_requests_total` - Total HTTP requests by method, status, endpoint
- `http_request_duration_seconds` - HTTP request duration histogram
- `files_processed_total` - Total files processed for deduplication
- `duplicates_found_total` - Total duplicate files found
- `deduplication_duration_seconds` - Time spent processing files
- `active_deduplication_jobs` - Number of active deduplication jobs
- `storage_bytes_saved_total` - Total bytes saved through deduplication

### System Metrics

- `process_cpu_seconds_total` - CPU usage
- `process_resident_memory_bytes` - Memory usage
- `up` - Service health status

## Log Collection

### Sources

1. **Docker Container Logs**: All container stdout/stderr
2. **System Logs**: `/var/log/syslog`
3. **Application Logs**: Custom application logs (if configured)

### Log Labels

- `job`: Source of the logs (containers, syslog, etc.)
- `container_name`: Docker container name
- `service`: Service identifier
- `level`: Log level (info, warn, error, debug)

## Configuration Details

### Environment Variables

The backend service uses these OpenTelemetry environment variables:

```bash
OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-collector:4317
OTEL_SERVICE_NAME=file-dedup-backend
OTEL_SERVICE_VERSION=0.1.0
OTEL_RESOURCE_ATTRIBUTES=service.name=file-dedup-backend,service.version=0.1.0
```

### Rust Integration

The Rust backend includes:

- OpenTelemetry tracing with OTLP export
- Prometheus metrics with custom business metrics
- HTTP request metrics middleware
- Structured JSON logging with tracing integration

## Customization

### Adding New Metrics

1. **In Rust Code**:

```rust
use observability::FileDeduplicationMetrics;

// In your handler
let metrics = req.app_data::<web::Data<Rc<FileDeduplicationMetrics>>>().unwrap();
metrics.files_processed_total.inc();
```

2. **Update Grafana Dashboard**:
   - Edit `grafana/dashboards/file-dedup-dashboard.json`
   - Or create new panels in Grafana UI

### Adding New Log Sources

1. **Update Promtail Config**:

```yaml
# In promtail/promtail-config.yml
- job_name: my-new-logs
  static_configs:
    - targets:
        - localhost
      labels:
        job: my-new-logs
        __path__: /path/to/logs/*.log
```

### Custom Dashboards

1. Create dashboards in Grafana UI
2. Export JSON and save to `grafana/dashboards/`
3. Restart Grafana to load new dashboards

## Troubleshooting

### Common Issues

1. **Metrics not appearing in Prometheus**:

   - Check OTEL Collector logs: `docker logs file-dedup-otel-collector`
   - Verify backend is sending metrics: `curl http://localhost:8080/metrics`

2. **Logs not appearing in Loki**:

   - Check Promtail logs: `docker logs file-dedup-promtail`
   - Verify Loki is receiving logs: `curl http://localhost:3100/ready`

3. **Grafana can't connect to data sources**:
   - Check network connectivity between containers
   - Verify data source URLs in Grafana settings

### Useful Commands

```bash
# Check service health
docker-compose ps

# View logs for specific service
docker logs file-dedup-prometheus
docker logs file-dedup-grafana
docker logs file-dedup-loki

# Restart observability stack
docker-compose restart prometheus grafana loki promtail otel-collector

# Check metrics endpoint
curl http://localhost:8080/metrics

# Query Prometheus directly
curl 'http://localhost:9090/api/v1/query?query=up'

# Check Loki logs
curl 'http://localhost:3100/loki/api/v1/query?query={job="containerlogs"}'
```

## Performance Considerations

### Resource Usage

- **Prometheus**: ~200MB RAM, stores 200h of metrics
- **Grafana**: ~100MB RAM
- **Loki**: ~150MB RAM
- **OTEL Collector**: ~50MB RAM
- **Promtail**: ~30MB RAM

### Retention Policies

- **Prometheus**: 200 hours (configurable in prometheus.yml)
- **Loki**: Default retention (configurable in loki-config.yml)

### Scaling

For production environments:

1. Use external storage for Prometheus (e.g., remote write to cloud)
2. Configure Loki with object storage backend
3. Set up Grafana with external database
4. Use multiple OTEL Collector instances for high availability

## Security Considerations

1. **Change default passwords** in production
2. **Enable authentication** for all services
3. **Use TLS/SSL** for external access
4. **Restrict network access** to observability services
5. **Configure proper retention policies** to manage storage

## Integration with CI/CD

### Health Checks

```bash
# Add to your deployment pipeline
curl -f http://localhost:9090/-/healthy  # Prometheus
curl -f http://localhost:3001/api/health # Grafana
curl -f http://localhost:3100/ready      # Loki
```

### Alerting

Configure Prometheus alerting rules and Grafana notifications for:

- High error rates
- Service downtime
- Resource exhaustion
- Deduplication job failures

## Next Steps

1. **Set up alerting** with Prometheus AlertManager
2. **Add distributed tracing** with Jaeger
3. **Implement SLOs/SLIs** for service reliability
4. **Create runbooks** for common operational scenarios
5. **Set up log-based alerting** in Grafana
