# File Deduplication System

A production-ready file deduplication system built with Rust and React, featuring secure multi-user support, real-time processing, and comprehensive monitoring.

## 🏗️ System Architecture

### Frontend (React)

- **Secure Authentication**: JWT tokens with proper session management
- **Real-time Updates**: WebSocket connections for job status notifications
- **Result Visualization**: Interactive displays of deduplication results
- **Client-side Security**: Rate limiting and input validation

### Backend (Rust)

- **Async Architecture**: Built on Actix-web for high performance
- **Job Queue System**: Redis-powered queue for long-running tasks
- **Deduplication Engine**: SHA-256 hashing + OpenSearch based for near duplication
- **Multi-tenancy**: Complete user isolation and data security

## 🔒 Security Measures

### Authentication & Authorization

- **JWT Authentication**: Secure token generation and validation
- **Password Security**: bcrypt hashing with salt

### Infrastructure Security

- **Transport Security**: HTTPS with TLS 1.2+
- **Security Headers**: CSP, HSTS, X-Frame-Options
- **Reverse Proxy**: Nginx with request filtering
- **Container Security**: Resource limits and isolation
- **Intrusion Prevention**: Fail2ban integration

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

## 🐳 Production Deployment

### Docker Infrastructure

- **Optimized Images**: Multi-stage builds for minimal size
- **Data Persistence**: PostgreSQL with volume mounting
- **Caching Layer**: Redis for queues and session storage
- **SSL Termination**: Nginx reverse proxy with HTTPS
- **Monitoring Stack**: Prometheus and Grafana integration

### Security Hardening

- **Network Security**: Firewall configuration and rules
- **Certificate Management**: SSL with automatic renewal
- **Intrusion Detection**: Fail2ban monitoring and response
- **Security Policies**: Complete CSP and security headers
- **Resource Controls**: Container limits and quotas

### Monitoring & Alerting

- **Application Metrics**:
  - Request rates and response times
  - Error rates and queue depth
  - User activity and job completion rates
- **System Metrics**:
  - CPU, memory, and disk utilization
  - Network traffic and latency
  - Container health and resource usage
- **Security Metrics**:
  - Failed authentication attempts
  - Rate limit violations
  - Malware detection events
- **Visualization**: Grafana dashboards with configurable alerts

## 🚀 Key Features

| Feature                      | Description                                                |
| ---------------------------- | ---------------------------------------------------------- |
| **Secure File Upload**       | Multi-file upload with comprehensive security scanning     |
| **Async Processing**         | Non-blocking deduplication jobs with queue management      |
| **Real-time Updates**        | WebSocket notifications for job progress and completion    |
| **Multi-tenancy**            | Complete user isolation - users see only their data        |
| **Storage Management**       | Per-user quotas with automatic cleanup policies            |
| **Audit Logging**            | Complete audit trail of all user actions and system events |
| **Rate Limiting**            | Multi-level protection against abuse and DoS attacks       |
| **Comprehensive Monitoring** | Full metrics collection and alerting system                |

## 📊 Observability & Monitoring

The system includes a comprehensive observability stack using OpenTelemetry, Prometheus, Grafana, and Loki:

### 🚀 Quick Start with Observability

```bash
# Start the complete stack with observability
./start-observability.sh

# Or manually with docker-compose
docker-compose up -d
```

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
