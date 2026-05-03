# Sentinel-MCP Deployment Guide

Complete guide for deploying Sentinel-MCP in various environments.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Docker Deployment](#docker-deployment)
- [Kubernetes Deployment](#kubernetes-deployment)
- [Bare Metal Deployment](#bare-metal-deployment)
- [Cloud Deployments](#cloud-deployments)
- [Production Checklist](#production-checklist)
- [Monitoring Setup](#monitoring-setup)

## Prerequisites

### System Requirements

- **CPU**: 2+ cores recommended
- **RAM**: 2GB minimum, 4GB recommended
- **Disk**: 10GB minimum
- **OS**: Linux (Ubuntu 20.04+, RHEL 8+, or similar)

### Required Software

- Rust 1.75+ (for building from source)
- Docker 20.10+ (for containerized deployment)
- Kubernetes 1.24+ (for K8s deployment)
- Prometheus & AlertManager (for monitoring integration)

### IBM watsonx.ai Access

1. Create an IBM Cloud account
2. Set up watsonx.ai service
3. Obtain API key and project ID
4. Note your region URL

## Docker Deployment

### Quick Start

```bash
# Pull the image
docker pull ghcr.io/paulmmoore3416/sentinel-mcp:latest

# Run with environment variables
docker run -d \
  --name sentinel-mcp \
  -p 3000:3000 \
  -p 9090:9090 \
  -e WATSONX_API_KEY=your_api_key \
  -e WATSONX_PROJECT_ID=your_project_id \
  -e SENTINEL_LOG_LEVEL=info \
  ghcr.io/paulmmoore3416/sentinel-mcp:latest
```

### Docker Compose

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  sentinel-mcp:
    image: ghcr.io/paulmmoore3416/sentinel-mcp:latest
    container_name: sentinel-mcp
    ports:
      - "3000:3000"  # API server
      - "9090:9090"  # Metrics
    environment:
      - WATSONX_API_KEY=${WATSONX_API_KEY}
      - WATSONX_PROJECT_ID=${WATSONX_PROJECT_ID}
      - WATSONX_URL=https://us-south.ml.cloud.ibm.com
      - SENTINEL_PORT=3000
      - SENTINEL_LOG_LEVEL=info
      - SENTINEL_DRY_RUN=false
    volumes:
      - ./config.yaml:/app/config.yaml:ro
      - ./plugins:/app/plugins:ro
      - sentinel-logs:/app/logs
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/api/v1/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

  prometheus:
    image: prom/prometheus:latest
    container_name: prometheus
    ports:
      - "9091:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - prometheus-data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
    restart: unless-stopped

  alertmanager:
    image: prom/alertmanager:latest
    container_name: alertmanager
    ports:
      - "9093:9093"
    volumes:
      - ./alertmanager.yml:/etc/alertmanager/alertmanager.yml:ro
    command:
      - '--config.file=/etc/alertmanager/alertmanager.yml'
    restart: unless-stopped

volumes:
  sentinel-logs:
  prometheus-data:
```

Start the stack:

```bash
docker-compose up -d
```

### Building Custom Image

```dockerfile
# Dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/sentinel-mcp /app/
COPY config.example.yaml /app/config.yaml

EXPOSE 3000 9090

CMD ["./sentinel-mcp", "start"]
```

Build and run:

```bash
docker build -t sentinel-mcp:custom .
docker run -d -p 3000:3000 sentinel-mcp:custom
```

## Kubernetes Deployment

### Namespace Setup

```bash
kubectl create namespace sentinel-system
```

### Secrets

Create secrets for sensitive data:

```bash
kubectl create secret generic watsonx-credentials \
  --from-literal=api-key=${WATSONX_API_KEY} \
  --from-literal=project-id=${WATSONX_PROJECT_ID} \
  -n sentinel-system
```

### ConfigMap

Create `configmap.yaml`:

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: sentinel-config
  namespace: sentinel-system
data:
  config.yaml: |
    server:
      host: "0.0.0.0"
      port: 3000
      workers: 4
    remediation:
      auto_approve_low_risk: true
      auto_approve_medium_risk: false
      dry_run_mode: false
    logging:
      level: "info"
      format: "json"
```

Apply:

```bash
kubectl apply -f configmap.yaml
```

### Deployment

Create `deployment.yaml`:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: sentinel-mcp
  namespace: sentinel-system
  labels:
    app: sentinel-mcp
spec:
  replicas: 2
  selector:
    matchLabels:
      app: sentinel-mcp
  template:
    metadata:
      labels:
        app: sentinel-mcp
    spec:
      serviceAccountName: sentinel-mcp
      containers:
      - name: sentinel-mcp
        image: ghcr.io/paulmmoore3416/sentinel-mcp:latest
        imagePullPolicy: Always
        ports:
        - name: http
          containerPort: 3000
          protocol: TCP
        - name: metrics
          containerPort: 9090
          protocol: TCP
        env:
        - name: WATSONX_API_KEY
          valueFrom:
            secretKeyRef:
              name: watsonx-credentials
              key: api-key
        - name: WATSONX_PROJECT_ID
          valueFrom:
            secretKeyRef:
              name: watsonx-credentials
              key: project-id
        - name: WATSONX_URL
          value: "https://us-south.ml.cloud.ibm.com"
        - name: SENTINEL_LOG_LEVEL
          value: "info"
        volumeMounts:
        - name: config
          mountPath: /app/config.yaml
          subPath: config.yaml
        resources:
          requests:
            memory: "512Mi"
            cpu: "250m"
          limits:
            memory: "2Gi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /api/v1/health/live
            port: 3000
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /api/v1/health/ready
            port: 3000
          initialDelaySeconds: 10
          periodSeconds: 5
      volumes:
      - name: config
        configMap:
          name: sentinel-config
```

### Service

Create `service.yaml`:

```yaml
apiVersion: v1
kind: Service
metadata:
  name: sentinel-mcp
  namespace: sentinel-system
  labels:
    app: sentinel-mcp
spec:
  type: ClusterIP
  ports:
  - name: http
    port: 3000
    targetPort: 3000
    protocol: TCP
  - name: metrics
    port: 9090
    targetPort: 9090
    protocol: TCP
  selector:
    app: sentinel-mcp
```

### ServiceAccount & RBAC

Create `rbac.yaml`:

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: sentinel-mcp
  namespace: sentinel-system
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: sentinel-mcp
rules:
- apiGroups: [""]
  resources: ["pods", "services", "configmaps"]
  verbs: ["get", "list", "watch"]
- apiGroups: ["apps"]
  resources: ["deployments", "statefulsets"]
  verbs: ["get", "list", "watch", "patch"]
- apiGroups: [""]
  resources: ["pods/log"]
  verbs: ["get"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: sentinel-mcp
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: sentinel-mcp
subjects:
- kind: ServiceAccount
  name: sentinel-mcp
  namespace: sentinel-system
```

### Deploy Everything

```bash
kubectl apply -f rbac.yaml
kubectl apply -f configmap.yaml
kubectl apply -f deployment.yaml
kubectl apply -f service.yaml
```

### Verify Deployment

```bash
# Check pods
kubectl get pods -n sentinel-system

# Check logs
kubectl logs -f deployment/sentinel-mcp -n sentinel-system

# Port forward for testing
kubectl port-forward svc/sentinel-mcp 3000:3000 -n sentinel-system
```

## Bare Metal Deployment

### System Setup

```bash
# Install dependencies
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Build from Source

```bash
# Clone repository
git clone https://github.com/paulmmoore3416/Sentinel-MCP.git
cd Sentinel-MCP

# Build release binary
cargo build --release

# Install binary
sudo cp target/release/sentinel-mcp /usr/local/bin/
sudo chmod +x /usr/local/bin/sentinel-mcp
```

### Configuration

```bash
# Create config directory
sudo mkdir -p /etc/sentinel-mcp
sudo cp config.example.yaml /etc/sentinel-mcp/config.yaml

# Edit configuration
sudo nano /etc/sentinel-mcp/config.yaml
```

### Systemd Service

Create `/etc/systemd/system/sentinel-mcp.service`:

```ini
[Unit]
Description=Sentinel-MCP Autonomous Infrastructure Repair Agent
After=network.target

[Service]
Type=simple
User=sentinel
Group=sentinel
WorkingDirectory=/opt/sentinel-mcp
ExecStart=/usr/local/bin/sentinel-mcp start --config /etc/sentinel-mcp/config.yaml
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/log/sentinel-mcp

[Install]
WantedBy=multi-user.target
```

Create user and directories:

```bash
# Create user
sudo useradd -r -s /bin/false sentinel

# Create directories
sudo mkdir -p /opt/sentinel-mcp/plugins
sudo mkdir -p /var/log/sentinel-mcp
sudo chown -R sentinel:sentinel /opt/sentinel-mcp
sudo chown -R sentinel:sentinel /var/log/sentinel-mcp
```

Enable and start service:

```bash
sudo systemctl daemon-reload
sudo systemctl enable sentinel-mcp
sudo systemctl start sentinel-mcp
sudo systemctl status sentinel-mcp
```

## Cloud Deployments

### AWS ECS

Use the provided `ecs-task-definition.json`:

```json
{
  "family": "sentinel-mcp",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "512",
  "memory": "1024",
  "containerDefinitions": [
    {
      "name": "sentinel-mcp",
      "image": "ghcr.io/paulmmoore3416/sentinel-mcp:latest",
      "portMappings": [
        {
          "containerPort": 3000,
          "protocol": "tcp"
        }
      ],
      "environment": [
        {
          "name": "WATSONX_URL",
          "value": "https://us-south.ml.cloud.ibm.com"
        }
      ],
      "secrets": [
        {
          "name": "WATSONX_API_KEY",
          "valueFrom": "arn:aws:secretsmanager:region:account:secret:watsonx-api-key"
        }
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/sentinel-mcp",
          "awslogs-region": "us-east-1",
          "awslogs-stream-prefix": "ecs"
        }
      }
    }
  ]
}
```

### Google Cloud Run

```bash
# Build and push image
gcloud builds submit --tag gcr.io/PROJECT_ID/sentinel-mcp

# Deploy
gcloud run deploy sentinel-mcp \
  --image gcr.io/PROJECT_ID/sentinel-mcp \
  --platform managed \
  --region us-central1 \
  --allow-unauthenticated \
  --set-env-vars WATSONX_API_KEY=your_key \
  --set-env-vars WATSONX_PROJECT_ID=your_project
```

### Azure Container Instances

```bash
az container create \
  --resource-group sentinel-rg \
  --name sentinel-mcp \
  --image ghcr.io/paulmmoore3416/sentinel-mcp:latest \
  --dns-name-label sentinel-mcp \
  --ports 3000 9090 \
  --environment-variables \
    WATSONX_API_KEY=your_key \
    WATSONX_PROJECT_ID=your_project \
  --cpu 2 \
  --memory 4
```

## Production Checklist

### Security

- [ ] Configure TLS/SSL certificates
- [ ] Set up authentication tokens
- [ ] Review and restrict allowed commands
- [ ] Enable audit logging
- [ ] Configure firewall rules
- [ ] Use secrets management (Vault, AWS Secrets Manager, etc.)

### High Availability

- [ ] Deploy multiple replicas (minimum 2)
- [ ] Configure load balancer
- [ ] Set up health checks
- [ ] Configure auto-scaling
- [ ] Implement circuit breakers
- [ ] Set up backup and recovery

### Monitoring

- [ ] Configure Prometheus scraping
- [ ] Set up Grafana dashboards
- [ ] Configure alerting rules
- [ ] Enable log aggregation
- [ ] Set up distributed tracing
- [ ] Configure uptime monitoring

### Performance

- [ ] Tune worker count based on CPU cores
- [ ] Configure appropriate resource limits
- [ ] Enable connection pooling
- [ ] Optimize database queries (if applicable)
- [ ] Configure caching
- [ ] Set up CDN for static assets

### Compliance

- [ ] Document data retention policies
- [ ] Configure audit trails
- [ ] Implement access controls
- [ ] Set up compliance reporting
- [ ] Review security policies
- [ ] Conduct security audit

## Monitoring Setup

### Prometheus Configuration

Create `prometheus.yml`:

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'sentinel-mcp'
    static_configs:
      - targets: ['sentinel-mcp:9090']
    metrics_path: '/metrics'
```

### Grafana Dashboard

Import the provided dashboard JSON from `grafana/sentinel-dashboard.json` or create custom panels:

**Key Metrics to Monitor:**
- Alert processing rate
- Remediation success rate
- Mean Time to Recovery (MTTR)
- Active remediations
- Queue length
- API response times
- Error rates

### AlertManager Rules

Create `alertmanager.yml`:

```yaml
route:
  receiver: 'sentinel-mcp'
  group_by: ['alertname', 'instance']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 1h

receivers:
  - name: 'sentinel-mcp'
    webhook_configs:
      - url: 'http://sentinel-mcp:3000/api/v1/alerts'
        send_resolved: true
```

## Troubleshooting

### Common Issues

**Service won't start:**
```bash
# Check logs
journalctl -u sentinel-mcp -f

# Verify configuration
sentinel-mcp config validate

# Check permissions
ls -la /etc/sentinel-mcp/config.yaml
```

**High memory usage:**
```bash
# Reduce worker count in config
# Implement rate limiting
# Check for memory leaks in logs
```

**Slow response times:**
```bash
# Check watsonx.ai API latency
# Increase timeout values
# Scale horizontally
```

## Support

For deployment issues:
- GitHub Issues: https://github.com/paulmmoore3416/Sentinel-MCP/issues
- Documentation: https://github.com/paulmmoore3416/Sentinel-MCP/docs

---

**Made with ❤️ using IBM Bob and watsonx.ai**