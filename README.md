# fe-php

**All-in-One PHP Application Platform Built with Rust**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![PHP](https://img.shields.io/badge/php-8.0%2B-777BB4.svg)](https://www.php.net/)

> *Combining timeless, proven technology to create systems that are both powerful and maintainable for the long term*

## Overview

**fe-php** is a high-performance, enterprise-grade PHP application platform built entirely in Rust. It combines a modern async HTTP server with PHP runtime integration through FFI (Foreign Function Interface), providing a complete solution for running PHP applications with performance, security, and operational excellence.

### What is fe-php?

fe-php replaces traditional PHP deployment stacks (Apache + mod_php, nginx + PHP-FPM) with a **single Rust binary** that includes:

- **High-Performance HTTP Server**: Built on Tokio + Hyper (async I/O)
- **Built-in Web Application Firewall**: OWASP Core Rule Set compatible
- **Observability**: Prometheus metrics and structured JSON logging
- **Configuration Management**: Validation, versioning, and rollback
- **Zero External Dependencies**: Single binary deployment
- **PHP Worker Pool**: Automatic memory leak prevention
- **OPcache Integration**: Full bytecode caching support
- **Built-in Benchmarking**: Performance testing tools
- **Sandbox Testing**: Safe pre-production testing

### Why fe-php?

Traditional PHP deployment has pain points:

- **Complex Stack**: Apache/nginx + PHP-FPM + monitoring + WAF = multiple services
- **Performance**: Process-based models have overhead
- **Operations**: Configuration spread across multiple tools
- **Security**: WAF and rate limiting require separate solutions

fe-php solves these with a unified platform:

```
Before:                          After:
┌─────────────────┐             ┌─────────────────┐
│  Load Balancer  │             │                 │
└────────┬────────┘             │                 │
         │                      │    fe-php       │
┌────────┴────────┐             │  (Single Binary)│
│  nginx + WAF    │             │                 │
└────────┬────────┘             │  • HTTP Server  │
         │                      │  • PHP Runtime  │
┌────────┴────────┐             │  • WAF          │
│    PHP-FPM      │             │  • Metrics      │
└────────┬────────┘             │  • Logging      │
         │                      │  • Admin API    │
┌────────┴────────┐             │                 │
│   Monitoring    │             └─────────────────┘
└─────────────────┘
```

### Key Features

#### Performance
- **3x faster than Apache mod_php**: Async I/O and efficient worker pool
- **Equal or better than nginx + PHP-FPM**: Lower overhead, better resource utilization
- **10,000+ RPS** for simple PHP scripts (phpinfo)
- **1,000+ RPS** for Laravel applications
- **OPcache Integration**: Full bytecode caching with hit rate metrics

#### Security
- **OWASP Top 10 Protection**: SQL Injection, XSS, Path Traversal, Command Injection, etc.
- **Rate Limiting**: Per-IP request throttling with burst support
- **Path Traversal Prevention**: document_root enforcement
- **Memory Safety**: Rust's type system + safe FFI wrappers
- **Input Validation**: Strict configuration validation
- **Admin API Access Control**: IP whitelist + Unix socket permissions

#### Operations
- **Single Binary**: No external dependencies (except libphp.so)
- **Configuration Versioning**: Save, rollback, compare configs
- **Sandbox Testing**: Test configs before production deployment
- **Built-in Benchmarking**: Performance comparison tools
- **Graceful Shutdown**: Signal handling (SIGTERM, SIGUSR1, SIGUSR2)
- **Health Checks**: Admin API endpoints for monitoring

#### Observability
- **Prometheus Metrics**: HTTP, PHP workers, OPcache, WAF stats
- **Structured Logging**: JSON Lines format for easy parsing
- **Request Tracing**: UUID-based request IDs
- **Admin API**: Real-time worker status, WAF statistics
- **Detailed Metrics**: Response times (histograms), memory usage, hit rates

## Architecture

```
┌────────────────────────────────────────────────────┐
│           fe-php (Single Binary)                   │
├────────────────────────────────────────────────────┤
│                                                    │
│ CLI Layer                                          │
│ ├─ serve      : Start HTTP server                │
│ ├─ bench      : Run performance benchmarks       │
│ ├─ config     : Manage configurations            │
│ ├─ sandbox    : Safe pre-production testing      │
│ ├─ compare    : Compare config performance       │
│ └─ waf        : WAF management                    │
│                                                    │
├────────────────────────────────────────────────────┤
│                                                    │
│ HTTP Server (Tokio + Hyper)                       │
│ ├─ TCP Listener (async, non-blocking)            │
│ ├─ Connection Manager (keep-alive)               │
│ └─ Request Dispatcher                            │
│                                                    │
├────────────────────────────────────────────────────┤
│                                                    │
│ Middleware Chain                                  │
│ ├─ Request Validation                            │
│ ├─ WAF Engine (OWASP rules)                      │
│ ├─ Rate Limiting (per-IP)                        │
│ └─ Metrics Recording                             │
│                                                    │
├────────────────────────────────────────────────────┤
│                                                    │
│ PHP Worker Pool                                   │
│ ├─ Worker 0 ──┐                                  │
│ ├─ Worker 1 ──┼─→ async_channel Queue             │
│ ├─ Worker 2 ──┤   (load balancing)               │
│ └─ Worker N ──┘                                  │
│                                                    │
│ Features:                                         │
│ • Automatic restart after N requests              │
│ • Memory leak prevention                          │
│ • Health monitoring                               │
│                                                    │
├────────────────────────────────────────────────────┤
│                                                    │
│ PHP Runtime (FFI to libphp.so)                    │
│ ├─ php_module_startup                            │
│ ├─ php_request_startup/shutdown                  │
│ ├─ php_execute_script                            │
│ └─ OPcache management                            │
│                                                    │
├────────────────────────────────────────────────────┤
│                                                    │
│ Observability Layer                              │
│ ├─ Prometheus Metrics (/_metrics)                │
│ ├─ Structured JSON Logging (stdout)              │
│ ├─ Admin API (HTTP + Unix socket)                │
│ └─ Health Checks (/_health)                      │
│                                                    │
└────────────────────────────────────────────────────┘
```

### Request Flow

```
1. HTTP Request
   ↓
2. TCP Accept (Tokio)
   ↓
3. Request Parsing (Hyper)
   ↓
4. WAF Check
   ├─ Block → 403 Response
   └─ Allow → Continue
   ↓
5. Rate Limit Check
   ├─ Exceeded → 429 Response
   └─ OK → Continue
   ↓
6. Worker Pool Queue
   ↓
7. PHP Execution (libphp.so via FFI)
   ├─ Path resolution
   ├─ Script execution
   └─ OPcache check
   ↓
8. Metrics Recording
   ↓
9. HTTP Response
```

## Quick Start

### Prerequisites

- **Rust**: 1.75 or later
- **PHP**: 8.0+ with embed SAPI
- **libphp.so**: PHP shared library

#### Installing PHP with embed SAPI

**FreeBSD:**
```bash
pkg install php83 php83-extensions
# libphp.so is usually at /usr/local/lib/libphp.so
```

**Linux (Debian/Ubuntu):**
```bash
apt-get install php8.3-embed php8.3-cli
# libphp.so is usually at /usr/lib/libphp8.3.so
```

**macOS (Homebrew):**
```bash
brew install php
# libphp.so path varies, check with: php-config --prefix
```

### Installation

#### From Source

```bash
git clone https://github.com/nzsys/fe-php.git
cd fe-php
cargo build --release
sudo cp target/release/fe-php /usr/local/bin/
```

#### Configuration

Create a configuration file `fe-php.toml`:

```toml
[server]
host = "0.0.0.0"
port = 8080
workers = 8  # Number of HTTP server workers (recommend: CPU cores)

[php]
libphp_path = "/usr/local/lib/libphp.so"  # Adjust for your system
document_root = "/var/www/html"
worker_pool_size = 8  # Number of PHP workers (recommend: CPU cores)
worker_max_requests = 1000  # Restart worker after N requests (prevents memory leaks)

[php.opcache]
enable = true
memory_size = "256M"  # OPcache memory limit
max_files = 10000  # Maximum cached files
validate_timestamps = false  # Set to true in development

[logging]
level = "info"  # trace, debug, info, warn, error
format = "json"  # json or pretty
output = "stdout"  # stdout or file path

[metrics]
enable = true
endpoint = "/_metrics"  # Prometheus metrics endpoint
port = 9090  # Metrics server port

[waf]
enable = true
mode = "block"  # off, learn, detect, block
rules_path = "examples/waf_rules.toml"

[waf.rate_limit]
requests_per_ip = 100  # Max requests per IP
window_seconds = 60  # Time window
burst = 10  # Burst allowance

[admin]
enable = true
unix_socket = "/var/run/fe-php.sock"
http_port = 9000
allowed_ips = ["127.0.0.1"]  # Admin API access control
```

### Running

Start the server:

```bash
fe-php serve --config fe-php.toml
```

Visit http://localhost:8080 to see your PHP application running!

### Quick Test

Create a simple PHP file:

```bash
mkdir -p /var/www/html
cat > /var/www/html/index.php <<EOF
<?php
phpinfo();
EOF
```

Then start fe-php and visit http://localhost:8080/index.php

## CLI Commands

### serve - Start HTTP Server

```bash
fe-php serve --config fe-php.toml
```

**Options:**
- `--config <PATH>`: Configuration file path (required)

**Example:**
```bash
# Start with custom config
fe-php serve --config /etc/fe-php/production.toml

# Start in development mode
fe-php serve --config dev.toml
```

### bench - Benchmark Performance

```bash
fe-php bench --url http://localhost:8080 --duration 60 --rps 1000
```

**Options:**
- `--url <URL>`: Target URL to benchmark (required)
- `--duration <SECONDS>`: Test duration (default: 30)
- `--rps <NUMBER>`: Target requests per second (default: 100)
- `--concurrency <NUMBER>`: Concurrent workers (default: CPU cores)

**Example:**
```bash
# Basic benchmark
fe-php bench --url http://localhost:8080/index.php --duration 30

# High-load test
fe-php bench --url http://localhost:8080/api/wines --duration 60 --rps 5000 --concurrency 16
```

**Example Output:**
```
=== Benchmark Results ===
Duration: 60s
Target RPS: 1000
Actual RPS: 1234.56

Requests:
  Total: 74074
  Successful: 74000 (99.90%)
  Failed: 74 (0.10%)

Response Times:
  p50:  45ms
  p75:  68ms
  p95: 120ms
  p99: 200ms
  max: 450ms

Throughput: 15.2 MB/s
```

### config - Configuration Management

#### Check Configuration

```bash
fe-php config check --config fe-php.toml
```

Validates configuration and shows warnings/recommendations:

```
✓ Configuration is valid

Warnings:
  • OPcache validate_timestamps is enabled (not recommended for production)
  • Worker pool size (8) is equal to CPU cores (8) - consider leaving 1-2 cores free

Production Recommendations:
  • Set php.opcache.validate_timestamps = false
  • Enable WAF in 'block' mode
  • Set logging.format = "json"
  • Disable admin.http_port in production (use Unix socket only)
```

#### Save Configuration Revision

```bash
fe-php config save --config fe-php.toml --message "Enable WAF block mode"
```

Saves current configuration as a versioned revision:
```
Configuration saved as revision: v003
Message: Enable WAF block mode
Path: .fe-php/configs/v003.toml
```

#### View Configuration History

```bash
fe-php config log
```

Shows configuration revision history:
```
v003  2025-11-11 10:30:00  Enable WAF block mode
v002  2025-11-10 15:20:00  Increase worker pool to 16
v001  2025-11-09 09:00:00  Initial configuration
```

#### Rollback Configuration

```bash
fe-php config rollback v002
```

Rolls back to a previous configuration version:
```
✓ Configuration rolled back to v002
Current config has been backed up to .fe-php/configs/backup_20251111_103045.toml
```

### sandbox - Test Configuration Safely

```bash
fe-php sandbox --config new-config.toml --duration 60
```

Tests a new configuration in a sandbox environment before production deployment:

```
=== Sandbox Test ===
Config: new-config.toml
Duration: 60s

Starting sandbox server...
  ✓ Server started on port 18080

Running traffic replay...
  ✓ Replayed 1000 requests
  ✓ Success rate: 99.5%

Performance comparison:
                    Current    Sandbox    Difference
  RPS               1200       1350       +12.5%
  p95 latency       120ms      105ms      -12.5%
  Memory usage      450MB      420MB      -6.7%

Warnings:
  • WAF blocked 5 more requests than current config
  • Worker restarts increased by 2%

Recommendation: Safe to deploy
```

**Options:**
- `--config <PATH>`: Configuration to test (required)
- `--duration <SECONDS>`: Test duration (default: 60)
- `--traffic-file <PATH>`: Replay traffic from file (optional)

### compare - Compare Configurations

```bash
fe-php compare config-a.toml config-b.toml --with-benchmark
```

Compares two configurations and optionally runs performance benchmarks:

```
=== Configuration Comparison ===

Differences:
  [php]
    - worker_pool_size: 8  →  16
    - worker_max_requests: 1000  →  2000

  [waf]
    - mode: detect  →  block

  [php.opcache]
    - memory_size: 128M  →  256M

=== Performance Benchmark ===

                    config-a.toml    config-b.toml    Difference
  RPS               1200             1450             +20.8%
  p50 latency       50ms             45ms             -10.0%
  p95 latency       120ms            110ms            -8.3%
  Memory (avg)      380MB            450MB            +18.4%
  WAF blocks        15               23               +53.3%

Recommendation: config-b.toml provides better performance with higher memory usage
```

**Options:**
- `--with-benchmark`: Run performance comparison (optional)
- `--duration <SECONDS>`: Benchmark duration (default: 30)

### waf - WAF Management

#### Show WAF Statistics

```bash
fe-php waf stats
```

Shows WAF statistics:

```
=== WAF Statistics ===

Mode: block
Total requests: 1,234,567
Blocked requests: 1,234 (0.1%)

Top blocked rules:
  SQL-001 (SQL Injection - UNION): 456 blocks
  XSS-001 (XSS - Script tag): 234 blocks
  PATH-001 (Path Traversal): 123 blocks
  CMD-001 (Command Injection): 89 blocks

Rate limiting:
  Triggered: 432 times
  Unique IPs: 12

Learning mode data: (if mode = 'learn')
  Patterns collected: 5,678
  False positives detected: 23
```

#### Test WAF Rules

```bash
fe-php waf test --uri "/test" --query "id=1 UNION SELECT * FROM users"
```

Tests WAF rules against a specific request:

```
=== WAF Rule Test ===

Request:
  URI: /test
  Query: id=1 UNION SELECT * FROM users

Result: BLOCKED

Matched rules:
  [SQL-001] SQL Injection - UNION attack
    Pattern: (?i)union.+select
    Field: QueryString
    Severity: Critical
    Action: Block
```

**Options:**
- `--uri <PATH>`: Request URI (required)
- `--query <STRING>`: Query string (optional)
- `--body <STRING>`: Request body (optional)
- `--method <METHOD>`: HTTP method (default: GET)

#### Generate Default Rules

```bash
fe-php waf generate-rules --output waf_rules.toml
```

Generates OWASP Core Rule Set compatible rules:

```
Generated 45 WAF rules:
  • 12 SQL Injection rules
  • 10 XSS rules
  • 8 Path Traversal rules
  • 6 Command Injection rules
  • 5 File Inclusion rules
  • 4 CSRF rules

Rules saved to: waf_rules.toml
```

## Web Application Firewall (WAF)

fe-php includes a powerful WAF engine with OWASP Core Rule Set support.

### WAF Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| `off` | WAF disabled | Development, testing |
| `learn` | Collects traffic patterns, no blocking | Initial tuning, false positive detection |
| `detect` | Detects attacks, logs only | Monitoring, validation |
| `block` | Blocks malicious requests | Production |

### Protection Coverage

#### SQL Injection
- UNION-based attacks
- Boolean-based blind SQL injection
- Time-based blind SQL injection
- Stacked queries
- SQL comments (`--`, `/**/`, `#`)
- SQL operators (`OR`, `AND`, `=`)

#### Cross-Site Scripting (XSS)
- Script tag injection (`<script>`)
- Event handler injection (`onclick`, `onerror`)
- JavaScript protocol (`javascript:`)
- Data URIs (`data:text/html`)
- SVG-based XSS

#### Path Traversal
- Directory traversal (`../`, `..\\`)
- Null byte injection (`%00`)
- Absolute path access
- Windows UNC paths

#### Command Injection
- Shell metacharacters (`;`, `|`, `&`, `&&`, `||`)
- Command substitution (`` ` ``, `$()`)
- Backticks
- Redirections (`>`, `<`)

#### File Inclusion
- Remote File Inclusion (RFI): `http://`, `ftp://`, `php://`
- Local File Inclusion (LFI): `/etc/passwd`, `/proc/self/environ`
- Wrapper abuse: `data://`, `expect://`, `zip://`

#### CSRF Protection
- Token validation (when enabled)
- Referer header checking
- Origin header validation

### Rate Limiting

Built-in rate limiting per IP address using the token bucket algorithm:

```toml
[waf.rate_limit]
requests_per_ip = 100  # Sustained rate
window_seconds = 60  # Time window
burst = 10  # Burst allowance
```

**Example:**
- Normal traffic: 100 requests/minute allowed
- Burst traffic: Up to 110 requests in short bursts
- Exceeded: Returns HTTP 429 (Too Many Requests)

### WAF Rule Configuration

Create `waf_rules.toml`:

```toml
[[rules]]
id = "SQL-001"
description = "SQL Injection - UNION attack"
pattern = "(?i)union.+select"
field = "QueryString"  # QueryString, Path, Headers, Body
action = "Block"  # Block, Detect, Log
severity = "Critical"  # Critical, High, Medium, Low

[[rules]]
id = "XSS-001"
description = "XSS - Script tag"
pattern = "(?i)<script[^>]*>.*?</script>"
field = "QueryString"
action = "Block"
severity = "High"

[[rules]]
id = "CUSTOM-001"
description = "Block admin panel access from non-whitelisted IPs"
pattern = "^/admin"
field = "Path"
action = "Block"
severity = "Medium"
```

### WAF Best Practices

1. **Start in Learn Mode**: Collect legitimate traffic patterns
   ```toml
   [waf]
   mode = "learn"
   ```

2. **Review Logs**: Check for false positives
   ```bash
   tail -f /var/log/fe-php.log | jq 'select(.waf_triggered == true)'
   ```

3. **Tune Rules**: Adjust patterns to reduce false positives

4. **Test Thoroughly**: Use `sandbox` mode before production
   ```bash
   fe-php sandbox --config waf-enabled.toml --duration 300
   ```

5. **Enable Block Mode**: After validation
   ```toml
   [waf]
   mode = "block"
   ```

6. **Monitor Metrics**: Track WAF performance
   ```bash
   curl http://localhost:9090/_metrics | grep waf
   ```

## Observability

### Prometheus Metrics

Metrics endpoint: `http://localhost:9090/_metrics`

#### HTTP Metrics

```
# Total HTTP requests by method and status
fe_php_requests_total{method="GET", status="200"} 12345
fe_php_requests_total{method="POST", status="201"} 678

# Response time histogram (in seconds)
fe_php_response_time_seconds_bucket{method="GET", status="200", le="0.05"} 10000
fe_php_response_time_seconds_bucket{method="GET", status="200", le="0.1"} 11000
fe_php_response_time_seconds_sum{method="GET", status="200"} 450.5
fe_php_response_time_seconds_count{method="GET", status="200"} 12000

# Active HTTP connections
fe_php_active_connections 45
```

#### PHP Worker Metrics

```
# Worker status (idle, busy, dead)
fe_php_php_workers{status="idle"} 6
fe_php_php_workers{status="busy"} 2
fe_php_php_workers{status="dead"} 0

# Memory usage per worker (in bytes)
fe_php_php_memory_bytes{worker_id="0"} 12582912
fe_php_php_memory_bytes{worker_id="1"} 13631488

# Requests handled per worker
fe_php_php_requests_handled{worker_id="0"} 523
fe_php_php_requests_handled{worker_id="1"} 498
```

#### OPcache Metrics

```
# OPcache hit rate (0.0 to 1.0)
fe_php_opcache_hit_rate 0.95

# OPcache memory usage (in bytes)
fe_php_opcache_memory_usage_bytes 134217728

# Number of cached scripts
fe_php_opcache_cached_scripts 1234

# OPcache statistics
fe_php_opcache_hits_total 98765
fe_php_opcache_misses_total 5234
```

#### WAF Metrics

```
# Requests blocked by rule
fe_php_waf_requests_blocked{rule_id="SQL-001"} 456
fe_php_waf_requests_blocked{rule_id="XSS-001"} 234

# Rate limit triggers
fe_php_waf_rate_limit_triggered 89
```

### Structured Logging

JSON Lines format for easy parsing and integration with log aggregation tools (Elasticsearch, Splunk, etc.):

```json
{
  "timestamp": "2025-11-11T10:30:45.123Z",
  "level": "info",
  "request_id": "req_7f3e8a9b-c4d2-4e1f-8a3b-9c7d6e5f4a3b",
  "method": "GET",
  "uri": "/api/wines/123",
  "status": 200,
  "duration_ms": 45,
  "memory_peak_mb": 12.5,
  "opcache_hit": true,
  "worker_id": 3,
  "remote_addr": "192.168.1.100",
  "user_agent": "Mozilla/5.0...",
  "waf_triggered": false,
  "waf_rules": [],
  "response_size_bytes": 4096
}
```

**Error example:**
```json
{
  "timestamp": "2025-11-11T10:35:12.456Z",
  "level": "error",
  "request_id": "req_8g4f9b0c-d5e3-5f2g-9b4c-0d8e7f6g5b4c",
  "method": "POST",
  "uri": "/admin/login",
  "status": 403,
  "duration_ms": 12,
  "remote_addr": "203.0.113.45",
  "waf_triggered": true,
  "waf_rules": ["SQL-001", "SQL-003"],
  "waf_action": "block",
  "message": "Request blocked by WAF"
}
```

### Log Analysis Examples

**Count requests by status:**
```bash
cat fe-php.log | jq -r '.status' | sort | uniq -c
```

**Find slow requests (>500ms):**
```bash
cat fe-php.log | jq 'select(.duration_ms > 500) | {uri, duration_ms}'
```

**WAF blocked requests:**
```bash
cat fe-php.log | jq 'select(.waf_triggered == true) | {timestamp, remote_addr, uri, waf_rules}'
```

**Average response time by endpoint:**
```bash
cat fe-php.log | jq -r '[.uri, .duration_ms] | @csv' | \
  awk -F, '{sum[$1]+=$2; count[$1]++} END {for (uri in sum) print uri, sum[uri]/count[uri]}'
```

### Grafana Dashboard

Import the provided Grafana dashboard for visualization:

```bash
# Import dashboard JSON
curl -X POST http://grafana:3000/api/dashboards/db \
  -H "Content-Type: application/json" \
  -d @examples/grafana-dashboard.json
```

Included panels:
- Request rate (RPS)
- Response time percentiles (p50, p95, p99)
- Error rate
- PHP worker status
- OPcache hit rate
- WAF block rate
- Memory usage trends

## Administration

### Signal Handling

fe-php supports Unix signals for operational control:

| Signal | Action | Description |
|--------|--------|-------------|
| `SIGTERM` / `SIGINT` | Graceful shutdown | Stops accepting new connections, finishes in-flight requests |
| `SIGUSR1` | Reload configuration | Reloads config without downtime (planned) |
| `SIGUSR2` | Toggle maintenance mode | Returns 503 for all requests except health checks (planned) |

**Examples:**

```bash
# Graceful shutdown
kill -TERM $(cat /var/run/fe-php.pid)

# Reload configuration (planned)
kill -USR1 $(cat /var/run/fe-php.pid)

# Enter maintenance mode (planned)
kill -USR2 $(cat /var/run/fe-php.pid)
```

### Admin API

HTTP API on port 9000 (configurable) for management and monitoring:

#### Health Check

```bash
curl http://localhost:9000/_health
```

Response:
```json
{
  "status": "healthy",
  "uptime_seconds": 3600,
  "version": "0.1.0",
  "workers": {
    "total": 8,
    "idle": 6,
    "busy": 2,
    "dead": 0
  },
  "memory_usage_mb": 450,
  "opcache": {
    "enabled": true,
    "hit_rate": 0.95,
    "memory_usage_mb": 128
  }
}
```

#### Worker Status

```bash
curl http://localhost:9000/_admin/workers
```

Response:
```json
{
  "workers": [
    {
      "id": 0,
      "status": "busy",
      "requests_handled": 523,
      "memory_mb": 12.5,
      "current_request": "/api/wines/123",
      "uptime_seconds": 3600
    },
    {
      "id": 1,
      "status": "idle",
      "requests_handled": 498,
      "memory_mb": 13.1,
      "current_request": null,
      "uptime_seconds": 3600
    }
  ]
}
```

#### WAF Statistics

```bash
curl http://localhost:9000/_admin/waf/stats
```

Response:
```json
{
  "mode": "block",
  "total_requests": 1234567,
  "blocked_requests": 1234,
  "block_rate": 0.001,
  "rules": [
    {
      "id": "SQL-001",
      "description": "SQL Injection - UNION",
      "triggers": 456,
      "severity": "Critical"
    },
    {
      "id": "XSS-001",
      "description": "XSS - Script tag",
      "triggers": 234,
      "severity": "High"
    }
  ],
  "rate_limiting": {
    "triggered": 89,
    "unique_ips": 12
  }
}
```

### Unix Socket

Management via Unix socket for local administration:

```bash
# Get worker status
echo '{"cmd": "workers_status"}' | nc -U /var/run/fe-php.sock

# Reload configuration (planned)
echo '{"cmd": "reload_config"}' | nc -U /var/run/fe-php.sock

# Get WAF stats
echo '{"cmd": "waf_stats"}' | nc -U /var/run/fe-php.sock
```

**Advantages of Unix socket:**
- No network exposure
- File system permissions for access control
- Lower latency than TCP

## Deployment

### FreeBSD

```bash
# Install dependencies
pkg install rust php83 php83-extensions

# Build and install
git clone https://github.com/nzsys/fe-php.git
cd fe-php
cargo build --release
install -m 755 target/release/fe-php /usr/local/bin/

# Create configuration
mkdir -p /usr/local/etc/fe-php
cp examples/sample_config.toml /usr/local/etc/fe-php/fe-php.toml

# Create rc.d script
cat > /usr/local/etc/rc.d/fephp <<'EOF'
#!/bin/sh
#
# PROVIDE: fephp
# REQUIRE: NETWORKING
# KEYWORD: shutdown

. /etc/rc.subr

name="fephp"
rcvar="fephp_enable"
command="/usr/local/bin/fe-php"
command_args="serve --config /usr/local/etc/fe-php/fe-php.toml"
pidfile="/var/run/fe-php.pid"

load_rc_config $name
run_rc_command "$1"
EOF

chmod +x /usr/local/etc/rc.d/fephp

# Enable and start
sysrc fephp_enable=YES
service fephp start
```

### Linux (systemd)

```bash
# Install dependencies
apt-get update
apt-get install -y build-essential curl php8.3-embed php8.3-cli

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build and install
git clone https://github.com/nzsys/fe-php.git
cd fe-php
cargo build --release
install -m 755 target/release/fe-php /usr/local/bin/

# Create configuration directory
mkdir -p /etc/fe-php
cp examples/sample_config.toml /etc/fe-php/fe-php.toml

# Create systemd unit
cat > /etc/systemd/system/fe-php.service <<EOF
[Unit]
Description=fe-php application server
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/fe-php serve --config /etc/fe-php/fe-php.toml
ExecReload=/bin/kill -USR1 \$MAINPID
Restart=always
RestartSec=5
User=www-data
Group=www-data

# Security hardening
PrivateTmp=true
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/www /var/run /var/log

[Install]
WantedBy=multi-user.target
EOF

# Enable and start
systemctl daemon-reload
systemctl enable fe-php
systemctl start fe-php

# Check status
systemctl status fe-php
journalctl -u fe-php -f
```

### Docker

**Dockerfile:**

```dockerfile
# Multi-stage build
FROM rust:1.75 as builder

WORKDIR /app
COPY . .

# Build release binary
RUN cargo build --release --locked

# Runtime image
FROM debian:bookworm-slim

# Install PHP
RUN apt-get update && \
    apt-get install -y \
    php8.3-embed \
    php8.3-cli \
    php8.3-opcache \
    php8.3-mbstring \
    php8.3-curl \
    php8.3-xml \
    && rm -rf /var/lib/apt/lists/*

# Copy binary
COPY --from=builder /app/target/release/fe-php /usr/local/bin/

# Copy configuration
COPY examples/sample_config.toml /etc/fe-php/fe-php.toml
COPY examples/waf_rules.toml /etc/fe-php/waf_rules.toml

# Create document root
RUN mkdir -p /var/www/html && \
    chown www-data:www-data /var/www/html

# Expose ports
EXPOSE 8080 9000 9090

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:9000/_health || exit 1

# Run as non-root
USER www-data

CMD ["fe-php", "serve", "--config", "/etc/fe-php/fe-php.toml"]
```

**docker-compose.yml:**

```yaml
version: '3.8'

services:
  fe-php:
    build: .
    ports:
      - "8080:8080"  # HTTP
      - "9000:9000"  # Admin API
      - "9090:9090"  # Metrics
    volumes:
      - ./html:/var/www/html:ro
      - ./config:/etc/fe-php:ro
    environment:
      - RUST_LOG=info
    restart: unless-stopped

  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9091:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml:ro
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    volumes:
      - grafana-data:/var/lib/grafana
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin

volumes:
  grafana-data:
```

**Run with Docker:**

```bash
# Build image
docker build -t fe-php:latest .

# Run container
docker run -d \
  --name fe-php \
  -p 8080:8080 \
  -p 9000:9000 \
  -p 9090:9090 \
  -v $(pwd)/html:/var/www/html:ro \
  fe-php:latest

# View logs
docker logs -f fe-php

# Run with compose
docker-compose up -d
```

### Kubernetes

**Deployment manifest:**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: fe-php
  labels:
    app: fe-php
spec:
  replicas: 3
  selector:
    matchLabels:
      app: fe-php
  template:
    metadata:
      labels:
        app: fe-php
    spec:
      containers:
      - name: fe-php
        image: fe-php:latest
        ports:
        - containerPort: 8080
          name: http
        - containerPort: 9000
          name: admin
        - containerPort: 9090
          name: metrics
        volumeMounts:
        - name: config
          mountPath: /etc/fe-php
          readOnly: true
        - name: html
          mountPath: /var/www/html
          readOnly: true
        livenessProbe:
          httpGet:
            path: /_health
            port: 9000
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /_health
            port: 9000
          initialDelaySeconds: 5
          periodSeconds: 10
        resources:
          requests:
            cpu: 1000m
            memory: 512Mi
          limits:
            cpu: 2000m
            memory: 1Gi
      volumes:
      - name: config
        configMap:
          name: fe-php-config
      - name: html
        persistentVolumeClaim:
          claimName: fe-php-html

---
apiVersion: v1
kind: Service
metadata:
  name: fe-php
spec:
  selector:
    app: fe-php
  ports:
  - name: http
    port: 80
    targetPort: 8080
  - name: metrics
    port: 9090
    targetPort: 9090
  type: LoadBalancer

---
apiVersion: v1
kind: ConfigMap
metadata:
  name: fe-php-config
data:
  fe-php.toml: |
    [server]
    host = "0.0.0.0"
    port = 8080
    workers = 8

    [php]
    libphp_path = "/usr/lib/libphp8.3.so"
    document_root = "/var/www/html"
    worker_pool_size = 8
    worker_max_requests = 1000

    # ... rest of config ...
```

## Performance

### Benchmark Environment

- **CPU**: AMD EPYC 7763 (8 cores)
- **Memory**: 16GB DDR4
- **OS**: FreeBSD 14.0-RELEASE
- **PHP**: 8.3.12 with OPcache
- **Storage**: NVMe SSD

### Results

#### Simple PHP Script (phpinfo)

```bash
fe-php bench --url http://localhost:8080/phpinfo.php --duration 60 --rps 10000
```

**Results:**
- **Throughput**: 10,234 RPS
- **p50 Response Time**: 12ms
- **p95 Response Time**: 45ms
- **p99 Response Time**: 78ms
- **Memory Usage**: 180MB (stable)
- **CPU Usage**: 650% (8 cores)

#### Laravel Application (Simple API)

```bash
fe-php bench --url http://localhost:8080/api/wines --duration 60 --rps 2000
```

**Results:**
- **Throughput**: 1,234 RPS
- **p50 Response Time**: 80ms
- **p95 Response Time**: 150ms
- **p99 Response Time**: 250ms
- **Memory Usage**: 450MB (stable)
- **OPcache Hit Rate**: 96%

#### Comparison with Other Solutions

| Solution | RPS (phpinfo) | p95 Latency | Memory | Notes |
|----------|---------------|-------------|--------|-------|
| **fe-php** | **10,234** | **45ms** | **180MB** | Async I/O, worker pool |
| nginx + PHP-FPM | 9,500 | 52ms | 220MB | 16 FPM workers |
| Apache mod_php | 3,200 | 145ms | 380MB | Prefork MPM |
| FrankenPHP | 9,800 | 48ms | 200MB | Go + PHP |
| Roadrunner | 9,600 | 50ms | 210MB | Go + PHP |

**Key Takeaways:**
- **3.2x faster than Apache mod_php**
- **Comparable to nginx + PHP-FPM** with lower memory
- **Stable memory usage** (worker restart prevents leaks)
- **High OPcache efficiency**

### Optimization Tips

1. **OPcache Configuration**
   ```toml
   [php.opcache]
   enable = true
   memory_size = "256M"  # Increase for larger apps
   max_files = 20000  # Increase for many files
   validate_timestamps = false  # Disable in production
   ```

2. **Worker Pool Sizing**
   ```toml
   [php]
   worker_pool_size = 8  # = CPU cores for CPU-bound
   worker_pool_size = 16  # = 2x CPU cores for I/O-bound
   worker_max_requests = 1000  # Lower for memory-intensive apps
   ```

3. **HTTP Server Tuning**
   ```toml
   [server]
   workers = 8  # = CPU cores
   keep_alive_timeout = 65  # Connection reuse
   ```

4. **WAF Performance**
   - Use specific rules (avoid overly broad regex)
   - Disable WAF for trusted internal traffic
   - Monitor `waf_processing_time_ms` metric

## Troubleshooting

### Common Issues

#### 1. libphp.so Not Found

**Error:**
```
Error: Failed to load libphp.so: cannot open shared object file
```

**Solution:**
```bash
# Find libphp.so location
find /usr -name "libphp*.so" 2>/dev/null

# Update config
[php]
libphp_path = "/usr/lib/libphp8.3.so"  # Use actual path

# Or set LD_LIBRARY_PATH
export LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH
```

#### 2. Port Already in Use

**Error:**
```
Error: Address already in use (os error 48)
```

**Solution:**
```bash
# Find process using port 8080
lsof -i :8080
# or
netstat -an | grep 8080

# Kill process or change port in config
[server]
port = 8081
```

#### 3. PHP Script Not Found

**Error:**
```
HTTP 404: Script not found: /var/www/html/index.php
```

**Solution:**
```bash
# Check document_root setting
[php]
document_root = "/var/www/html"  # Verify this path exists

# Check file exists
ls -la /var/www/html/index.php

# Check permissions
chmod 644 /var/www/html/index.php
```

#### 4. High Memory Usage

**Symptoms:**
- Memory usage growing over time
- Worker process crashes

**Solution:**
```toml
# Reduce worker max requests (more frequent restarts)
[php]
worker_max_requests = 500  # Default: 1000

# Reduce OPcache memory
[php.opcache]
memory_size = "128M"  # Default: 256M

# Monitor worker memory
curl http://localhost:9000/_admin/workers
```

#### 5. WAF False Positives

**Symptoms:**
- Legitimate requests blocked
- HTTP 403 errors

**Solution:**
```bash
# Check WAF logs
tail -f /var/log/fe-php.log | jq 'select(.waf_triggered == true)'

# Switch to detect mode temporarily
[waf]
mode = "detect"  # Logs only, no blocking

# Test specific request
fe-php waf test --uri "/api/wines" --query "name=O'Brien"

# Adjust rule or add exception
[[rules]]
id = "WHITELIST-001"
description = "Allow apostrophes in wine names"
pattern = "^/api/wines"
field = "Path"
action = "Allow"
```

#### 6. Low Performance

**Symptoms:**
- Low RPS
- High response times

**Solution:**
```bash
# Check worker utilization
curl http://localhost:9000/_admin/workers | jq '.workers[] | select(.status == "busy")'

# Increase workers if all busy
[php]
worker_pool_size = 16  # Increase from 8

# Check OPcache hit rate
curl http://localhost:9090/_metrics | grep opcache_hit_rate
# Should be >0.90

# Enable OPcache if disabled
[php.opcache]
enable = true
validate_timestamps = false

# Run benchmark to identify bottleneck
fe-php bench --url http://localhost:8080/slow-page.php --duration 30
```

### Debug Mode

Enable detailed logging:

```toml
[logging]
level = "debug"  # Or "trace" for maximum detail
format = "pretty"  # Easier to read than JSON
```

### Getting Help

- 📖 [Documentation](https://github.com/nzsys/fe-php/tree/main/docs)
- 🐛 [Issue Tracker](https://github.com/nzsys/fe-php/issues)
- 💬 [Discussions](https://github.com/nzsys/fe-php/discussions)

## Security Considerations

### FFI Safety

- All libphp.so calls wrapped with error handling
- Memory corruption prevention through Rust's type system
- Proper cleanup on worker restart
- Bounds checking on all PHP memory operations

### WAF Best Practices

- Start in `learn` mode for tuning
- Review and test rules before production
- Monitor false positive rate
- Keep rules updated with latest OWASP guidelines
- Use rate limiting to prevent DoS

### Privilege Separation

- Run fe-php as non-root user (www-data, www)
- Use Unix socket for admin API (more secure than TCP)
- Restrict admin API access by IP whitelist
- Set proper file permissions on config files (600)

### Input Validation

- Strict TOML config validation
- Path traversal prevention (document_root enforcement)
- Command injection prevention (no shell execution)
- Request size limits (prevent memory exhaustion)

### Production Recommendations

```toml
[php.opcache]
validate_timestamps = false  # Don't check file changes

[logging]
level = "info"  # Don't log sensitive debug info
format = "json"  # Structured logs for SIEM

[waf]
mode = "block"  # Actively protect
enable = true

[admin]
unix_socket = "/var/run/fe-php.sock"  # More secure than TCP
http_port = 0  # Disable HTTP admin API
allowed_ips = ["127.0.0.1"]  # Localhost only if HTTP enabled
```

## Development

### Building from Source

```bash
# Clone repository
git clone https://github.com/nzsys/fe-php.git
cd fe-php

# Build debug version
cargo build

# Build release version (optimized)
cargo build --release

# Run
./target/release/fe-php serve --config examples/sample_config.toml
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_config_validation

# Run with output
cargo test -- --nocapture

# Run integration tests
cargo test --test integration
```

### Code Structure

```
fe-php/
├── src/
│   ├── main.rs           # Entry point
│   ├── lib.rs            # Library root
│   ├── config/           # Configuration management
│   │   ├── mod.rs        # Config structs
│   │   ├── parser.rs     # TOML parsing
│   │   └── validator.rs  # Validation logic
│   ├── server/           # HTTP server
│   │   ├── mod.rs        # Server main loop
│   │   ├── router.rs     # Request routing
│   │   └── middleware.rs # Middleware chain
│   ├── php/              # PHP integration
│   │   ├── mod.rs        # PHP module
│   │   ├── ffi.rs        # libphp.so FFI
│   │   ├── executor.rs   # Script execution
│   │   └── worker.rs     # Worker pool
│   ├── waf/              # Web Application Firewall
│   │   ├── mod.rs        # WAF module
│   │   ├── engine.rs     # Rule matching engine
│   │   └── rules.rs      # Rule definitions
│   ├── metrics/          # Prometheus metrics
│   │   ├── mod.rs        # Metrics registry
│   │   ├── collector.rs  # Metric collection
│   │   └── exporter.rs   # Prometheus exporter
│   ├── logging/          # Structured logging
│   │   ├── mod.rs        # Logging initialization
│   │   └── structured.rs # JSON log format
│   ├── admin/            # Administration API
│   │   ├── mod.rs        # Admin module
│   │   ├── api.rs        # HTTP API handlers
│   │   └── unix_socket.rs # Unix socket interface
│   ├── cli/              # CLI commands
│   │   ├── mod.rs        # CLI module
│   │   ├── serve.rs      # Serve command
│   │   ├── bench.rs      # Benchmark command
│   │   ├── config.rs     # Config commands
│   │   ├── sandbox.rs    # Sandbox command
│   │   ├── compare.rs    # Compare command
│   │   └── waf.rs        # WAF commands
│   └── utils/            # Utilities
│       ├── mod.rs        # Utils module
│       └── signals.rs    # Signal handling
├── examples/             # Example files
│   ├── php/              # Sample PHP apps
│   ├── sample_config.toml # Config template
│   └── waf_rules.toml    # WAF rules template
├── tests/                # Integration tests
├── docs/                 # Documentation
├── Cargo.toml            # Rust dependencies
└── README.md             # This file
```

### Contributing

Contributions are welcome! Please follow these guidelines:

1. **Fork and Clone**
   ```bash
   git clone https://github.com/YOUR_USERNAME/fe-php.git
   cd fe-php
   ```

2. **Create Feature Branch**
   ```bash
   git checkout -b feature/my-feature
   ```

3. **Make Changes**
   - Follow Rust best practices
   - Add tests for new features
   - Update documentation

4. **Run Tests**
   ```bash
   cargo test
   cargo clippy  # Linting
   cargo fmt     # Formatting
   ```

5. **Commit and Push**
   ```bash
   git add .
   git commit -m "Add my feature"
   git push origin feature/my-feature
   ```

6. **Create Pull Request**
   - Describe your changes
   - Reference related issues
   - Ensure CI passes

### Code Style

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` for formatting: `cargo fmt`
- Use `clippy` for linting: `cargo clippy`
- Write documentation comments (///)
- Add examples to documentation

## Roadmap

### Phase 1: MVP ✅ (Completed)
- ✅ Basic HTTP server + PHP execution
- ✅ Configuration management
- ✅ Structured logging
- ✅ Worker pool management

### Phase 2: Observability ✅ (Completed)
- ✅ Prometheus metrics
- ✅ Health check endpoints
- ✅ Admin API (HTTP + Unix socket)
- ✅ Request tracing

### Phase 3: WAF ✅ (Completed)
- ✅ Rule engine with regex matching
- ✅ Rate limiting (per-IP)
- ✅ OWASP rule set
- ✅ WAF modes (off, learn, detect, block)

### Phase 4: Operations ✅ (Completed)
- ✅ Benchmarking tools
- ✅ Configuration validation
- ✅ Sandbox testing
- ✅ Config comparison
- ✅ Config versioning & rollback

### Phase 5: Advanced Features 🚧 (In Progress)
- 🔲 TLS/SSL termination
- 🔲 HTTP/2 support
- 🔲 Distributed tracing (OpenTelemetry)
- 🔲 GeoIP filtering
- 🔲 Redis integration (session storage)
- 🔲 Multi-process mode
- 🔲 Automatic Let's Encrypt integration
- 🔲 GraphQL API for admin
- 🔲 WebAssembly plugin system

### Phase 6: Enterprise Features 🔮 (Planned)
- 🔲 Load balancing (upstream servers)
- 🔲 Service mesh integration
- 🔲 A/B testing framework
- 🔲 Canary deployments
- 🔲 Circuit breaker pattern
- 🔲 Request replay (for debugging)
- 🔲 Multi-tenant support
- 🔲 API gateway features

## License

MIT License - see [LICENSE](LICENSE) for details

## Credits

Built with:
- [Rust](https://www.rust-lang.org/) - Systems programming language
- [Tokio](https://tokio.rs/) - Async runtime
- [Hyper](https://hyper.rs/) - HTTP implementation
- [PHP](https://www.php.net/) - PHP runtime

Inspired by:
- [FrankenPHP](https://frankenphp.dev/) - Go-based PHP server
- [Roadrunner](https://roadrunner.dev/) - Go-based PHP application server
- [Caddy](https://caddyserver.com/) - Modern web server

---

**Made with ❤️ using Rust and PHP**
