# Sentinel-MCP API Documentation

Complete API reference for Sentinel-MCP server endpoints.

## Table of Contents

- [Authentication](#authentication)
- [Alert Endpoints](#alert-endpoints)
- [Health & Status](#health--status)
- [Metrics](#metrics)
- [WebSocket](#websocket)
- [Plugin Management](#plugin-management)
- [Configuration](#configuration)

## Authentication

All API endpoints support optional Bearer token authentication.

```bash
curl -H "Authorization: Bearer YOUR_TOKEN" http://localhost:3000/api/v1/health
```

Configure authentication in `config.yaml`:

```yaml
security:
  auth_token: "your_secure_token_here"
```

## Alert Endpoints

### POST /api/v1/alerts

Receive alerts from Prometheus AlertManager or other monitoring systems.

**Request Body:**

```json
{
  "version": "4",
  "groupKey": "{}:{alertname=\"DiskSpaceLow\"}",
  "status": "firing",
  "receiver": "sentinel-mcp",
  "groupLabels": {},
  "commonLabels": {
    "alertname": "DiskSpaceLow",
    "severity": "warning"
  },
  "commonAnnotations": {
    "summary": "Disk space is low",
    "description": "Filesystem /var is at 92% capacity"
  },
  "externalURL": "http://alertmanager:9093",
  "alerts": [
    {
      "status": "firing",
      "labels": {
        "alertname": "DiskSpaceLow",
        "severity": "warning",
        "instance": "server-01",
        "filesystem": "/var"
      },
      "annotations": {
        "summary": "Disk space is critically low",
        "description": "Filesystem /var is at 92% capacity on server-01"
      },
      "startsAt": "2024-01-01T12:00:00Z",
      "endsAt": null,
      "generatorURL": "http://prometheus:9090/graph",
      "fingerprint": "abc123def456"
    }
  ]
}
```

**Response:**

```json
{
  "received": 1,
  "status": "success",
  "message": "Processed 1 alerts"
}
```

**Status Codes:**
- `200 OK` - Alerts received and queued
- `400 Bad Request` - Invalid alert format
- `401 Unauthorized` - Missing or invalid auth token
- `500 Internal Server Error` - Processing error

## Health & Status

### GET /api/v1/health

Get overall system health status.

**Response:**

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 3600,
  "components": {
    "api_server": {
      "status": "healthy",
      "message": null,
      "last_check": "2024-01-01T12:00:00Z"
    },
    "websocket_server": {
      "status": "healthy",
      "message": null,
      "last_check": "2024-01-01T12:00:00Z"
    },
    "watsonx_connection": {
      "status": "healthy",
      "message": "API responding",
      "last_check": "2024-01-01T12:00:00Z"
    },
    "plugin_system": {
      "status": "healthy",
      "message": "3 plugins loaded",
      "last_check": "2024-01-01T12:00:00Z"
    }
  }
}
```

**Status Codes:**
- `200 OK` - System is healthy or degraded
- `503 Service Unavailable` - System is unhealthy

### GET /api/v1/health/live

Kubernetes liveness probe endpoint.

**Response:**

```json
{
  "status": "alive",
  "timestamp": "2024-01-01T12:00:00Z"
}
```

### GET /api/v1/health/ready

Kubernetes readiness probe endpoint.

**Response:**

```json
{
  "ready": true,
  "status": "healthy",
  "timestamp": "2024-01-01T12:00:00Z"
}
```

**Status Codes:**
- `200 OK` - Service is ready
- `503 Service Unavailable` - Service is not ready

### GET /api/v1/status

Get current operational status.

**Response:**

```json
{
  "queue_length": 2,
  "active_remediations": 1,
  "total_processed": 42,
  "total_successful": 40,
  "total_failed": 2
}
```

## Metrics

### GET /metrics

Prometheus-compatible metrics endpoint.

**Response (text/plain):**

```
# TYPE sentinel_alerts_received_total counter
sentinel_alerts_received_total 42

# TYPE sentinel_remediations_executed_total counter
sentinel_remediations_executed_total 40

# TYPE sentinel_remediations_successful_total counter
sentinel_remediations_successful_total 38

# TYPE sentinel_remediations_failed_total counter
sentinel_remediations_failed_total 2

# TYPE sentinel_active_remediations gauge
sentinel_active_remediations 1

# TYPE sentinel_queue_length gauge
sentinel_queue_length 2

# TYPE sentinel_mttr_seconds histogram
sentinel_mttr_seconds_count 38
sentinel_mttr_seconds_sum 4560.5
sentinel_mttr_seconds_bucket{le="0.5"} 120.5
sentinel_mttr_seconds_bucket{le="0.95"} 180.2
sentinel_mttr_seconds_bucket{le="0.99"} 200.1
sentinel_mttr_seconds_bucket{le="+Inf"} 38

# TYPE sentinel_uptime_seconds gauge
sentinel_uptime_seconds 3600
```

### GET /metrics/summary

Get metrics summary in JSON format.

**Response:**

```json
{
  "uptime_seconds": 3600,
  "alerts_received": 42,
  "remediations_executed": 40,
  "remediations_successful": 38,
  "remediations_failed": 2,
  "active_remediations": 1,
  "queue_length": 2,
  "mttr_stats": {
    "count": 38,
    "sum": 4560.5,
    "mean": 120.0,
    "min": 45.2,
    "max": 300.5,
    "p50": 120.5,
    "p95": 180.2,
    "p99": 200.1
  },
  "api_calls": 156
}
```

## WebSocket

### WS /ws

Real-time notification stream via WebSocket.

**Connection:**

```javascript
const ws = new WebSocket('ws://localhost:3000/ws');

ws.onopen = () => {
  console.log('Connected to Sentinel-MCP');
};

ws.onmessage = (event) => {
  const notification = JSON.parse(event.data);
  console.log('Notification:', notification);
};
```

**Notification Types:**

#### AlertReceived

```json
{
  "type": "AlertReceived",
  "data": {
    "alert_id": "abc123",
    "alert_name": "DiskSpaceLow",
    "severity": "warning",
    "timestamp": "2024-01-01T12:00:00Z"
  }
}
```

#### RemediationStarted

```json
{
  "type": "RemediationStarted",
  "data": {
    "remediation_id": "rem-456",
    "alert_name": "DiskSpaceLow",
    "estimated_duration": 120
  }
}
```

#### RemediationProgress

```json
{
  "type": "RemediationProgress",
  "data": {
    "remediation_id": "rem-456",
    "step": 2,
    "total_steps": 3,
    "description": "Cleaning old log files",
    "status": "executing"
  }
}
```

#### RemediationCompleted

```json
{
  "type": "RemediationCompleted",
  "data": {
    "remediation_id": "rem-456",
    "success": true,
    "duration_ms": 115000,
    "message": "Disk space recovered successfully"
  }
}
```

#### HealthUpdate

```json
{
  "type": "HealthUpdate",
  "data": {
    "status": "healthy",
    "active_remediations": 0,
    "queue_length": 0,
    "metrics": {
      "cpu_usage": "45%",
      "memory_usage": "2.1GB"
    }
  }
}
```

#### Error

```json
{
  "type": "Error",
  "data": {
    "error_type": "RemediationExecution",
    "message": "Failed to execute command",
    "timestamp": "2024-01-01T12:00:00Z"
  }
}
```

**Client Commands:**

Subscribe to specific topics:

```json
{
  "command": "Subscribe",
  "topics": ["alerts", "remediations"]
}
```

Unsubscribe from topics:

```json
{
  "command": "Unsubscribe",
  "topics": ["alerts"]
}
```

Get status:

```json
{
  "command": "GetStatus"
}
```

## Plugin Management

### GET /api/v1/plugins

List all installed plugins.

**Response:**

```json
{
  "plugins": [
    {
      "name": "disk-cleanup",
      "version": "1.0.0",
      "author": "Sentinel-MCP",
      "description": "Handles disk space alerts by cleaning up old files",
      "supported_alert_types": ["DiskSpaceLow", "DiskSpaceCritical"]
    },
    {
      "name": "service-restart",
      "version": "1.0.0",
      "author": "Sentinel-MCP",
      "description": "Handles service failures by restarting services",
      "supported_alert_types": ["ServiceDown", "ServiceCrash"]
    }
  ]
}
```

### GET /api/v1/plugins/:name

Get information about a specific plugin.

**Response:**

```json
{
  "name": "disk-cleanup",
  "version": "1.0.0",
  "author": "Sentinel-MCP",
  "description": "Handles disk space alerts by cleaning up old files",
  "supported_alert_types": ["DiskSpaceLow", "DiskSpaceCritical"],
  "enabled": true,
  "loaded_at": "2024-01-01T10:00:00Z"
}
```

## Configuration

### GET /api/v1/config

Get current configuration (sensitive values redacted).

**Response:**

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 3000,
    "workers": 4
  },
  "remediation": {
    "auto_approve_low_risk": true,
    "auto_approve_medium_risk": false,
    "dry_run_mode": false
  },
  "plugins": {
    "enabled": true,
    "enabled_plugins": ["disk-cleanup", "service-restart"]
  }
}
```

### POST /api/v1/config/reload

Reload configuration from file without restarting.

**Response:**

```json
{
  "status": "success",
  "message": "Configuration reloaded successfully",
  "timestamp": "2024-01-01T12:00:00Z"
}
```

**Status Codes:**
- `200 OK` - Configuration reloaded
- `400 Bad Request` - Invalid configuration
- `500 Internal Server Error` - Reload failed

## Error Responses

All endpoints may return error responses in this format:

```json
{
  "error": {
    "type": "ValidationError",
    "message": "Invalid alert format",
    "details": "Missing required field: alertname",
    "timestamp": "2024-01-01T12:00:00Z"
  }
}
```

## Rate Limiting

API endpoints are rate-limited to prevent abuse:

- **Default**: 100 requests per minute per IP
- **Alerts endpoint**: 1000 requests per minute
- **WebSocket**: 10 connections per IP

Rate limit headers:

```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1640995200
```

## Examples

### cURL Examples

**Send a test alert:**

```bash
curl -X POST http://localhost:3000/api/v1/alerts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d @examples/alerts/disk-space-low.json
```

**Check health:**

```bash
curl http://localhost:3000/api/v1/health
```

**Get metrics:**

```bash
curl http://localhost:3000/metrics
```

### Python Example

```python
import requests
import json

# Send alert
alert = {
    "alerts": [{
        "status": "firing",
        "labels": {
            "alertname": "DiskSpaceLow",
            "severity": "warning",
            "filesystem": "/var"
        },
        "annotations": {
            "summary": "Disk space low",
            "description": "Filesystem /var at 92%"
        },
        "startsAt": "2024-01-01T12:00:00Z"
    }]
}

response = requests.post(
    "http://localhost:3000/api/v1/alerts",
    headers={"Content-Type": "application/json"},
    json=alert
)

print(response.json())
```

### JavaScript/Node.js Example

```javascript
const WebSocket = require('ws');

const ws = new WebSocket('ws://localhost:3000/ws');

ws.on('open', () => {
  console.log('Connected');
  
  // Subscribe to specific events
  ws.send(JSON.stringify({
    command: 'Subscribe',
    topics: ['remediations']
  }));
});

ws.on('message', (data) => {
  const event = JSON.parse(data);
  console.log('Event:', event.type, event.data);
});
```

## Support

For issues or questions:
- GitHub Issues: https://github.com/paulmmoore3416/Sentinel-MCP/issues
- Documentation: https://github.com/paulmmoore3416/Sentinel-MCP/docs

---

**Made with ❤️ using IBM Bob and watsonx.ai**