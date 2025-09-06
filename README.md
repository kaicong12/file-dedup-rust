# File Deduplication System

A production-ready file deduplication system built with Rust and React, featuring secure multi-user support, real-time processing, and comprehensive monitoring.

## 🏗️ System Architecture

### Frontend (React)

- **Secure Authentication**: JWT tokens with proper session management
- **Real-time Updates**: WebSocket connections for job status notifications
- **Result Visualization**: Interactive displays of deduplication results

### Backend (Rust)

- **Async Architecture**: Built on Actix-web for high performance
- **Job Queue System**: Redis-powered queue for long-running tasks
- **Deduplication Engine**: SHA-256 hashing + OpenSearch based for near duplication

## 🔒 Security Measures

### Authentication & Authorization

- **JWT Authentication**: Secure token generation and validation
- **Password Security**: bcrypt hashing with salt

## ⚡ Performance & Scalability

### Job Queue System

- **Async Processing**: Redis-backed queue system
- **Parallel Processing**: Multiple worker instances
- **Error Handling**: Dead letter queues for failed jobs
- **Priority Management**: Separate queues for admin tasks
- **Real-time Feedback**: WebSocket progress updates

### Database Optimization

- **Query Performance**: Comprehensive indexing strategy
- **Connection Management**: Pooling for efficiency
- **Data Lifecycle**: Automated cleanup of old data
- **Resource Management**: Per-user storage quotas

### Horizontal Scaling

- **Load Balancing**: Multiple backend instances
- **Worker Scaling**: Configurable worker processes
- **Proxy Configuration**: Nginx upstream load balancing
- **Orchestration**: Docker Compose for container management

## 📊 Observability & Monitoring

The system includes a comprehensive observability stack using OpenTelemetry, Prometheus, Grafana, and Loki:

### 📈 Monitoring Components

- **OpenTelemetry Collector**: Receives and processes telemetry data (metrics, logs, traces)
- **Prometheus**: Time-series database for metrics storage and alerting
- **Grafana**: Rich visualization dashboards and alerting (http://localhost:3001)
- **Loki**: Log aggregation and querying system
- **Promtail**: Log collection agent for Docker containers and system logs

### 🎯 Key Metrics Available

#### Application Metrics

- `http_requests_total` - HTTP requests by method, status, endpoint
- `http_request_duration_seconds` - Request latency histograms
- `files_processed_total` - Files processed for deduplication
- `duplicates_found_total` - Duplicate files identified
- `deduplication_duration_seconds` - Processing time metrics
- `active_deduplication_jobs` - Current job queue depth
- `storage_bytes_saved_total` - Storage efficiency metrics

#### System Metrics

- CPU and memory utilization
- Database connection pool status
- Redis queue metrics
- Container health and resource usage

### 📊 Access Points

- **Grafana Dashboard**: http://localhost:3001 (admin/admin)
- **Prometheus UI**: http://localhost:9090
- **Application Metrics**: http://localhost:8080/metrics
- **Pre-built Dashboard**: "File Deduplication Service Dashboard" in Grafana

### 🔧 Configuration

All observability configuration is located in the `observability/` directory:

- Prometheus scraping configuration
- Grafana datasources and dashboards
- OpenTelemetry Collector pipelines
- Loki log aggregation rules
- Promtail log collection patterns

See `observability/README.md` for detailed configuration and customization options.
