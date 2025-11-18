# Admin API Reference

fe-phpのAdmin APIの詳細なリファレンスです。

## 概要

Admin APIは2つのプロトコルをサポートしています：

1. **Unix Socket**: ローカルでの管理操作（推奨）
2. **HTTP JSON API**: 外部ツールとの連携用

## 認証とセキュリティ

### Unix Socket

Unix Socketはファイルパーミッションで保護されます。

設定:
```toml
[admin]
enable = true
unix_socket = "/var/run/fe-php-admin.sock"
```

パーミッション:
```bash
# 所有者のみアクセス可能
chmod 600 /var/run/fe-php-admin.sock
chown fe-php:fe-php /var/run/fe-php-admin.sock
```

### HTTP API

HTTP APIはIPアドレスフィルタリングで保護されます。

設定:
```toml
[admin]
enable = true
host = "127.0.0.1"
http_port = 9001
allowed_ips = ["127.0.0.1", "::1", "10.0.0.0/8"]
```

## エンドポイント一覧

### GET /api/status

サーバーの状態とメトリクスを取得します。

#### リクエスト

```bash
# HTTP
curl http://localhost:9001/api/status

# Unix Socket
echo '{"command":"status"}' | socat - UNIX-CONNECT:/var/run/fe-php-admin.sock
```

#### レスポンス

```json
{
  "server": {
    "version": "0.1.0",
    "uptime_seconds": 3600,
    "pid": 12345,
    "started_at": 1700000000
  },
  "metrics": {
    "requests_per_second": 150.5,
    "active_connections": 25,
    "total_requests": 541800,
    "error_rate": 2.5
  },
  "backends": [
    {
      "name": "Embedded (libphp)",
      "backend_type": "embedded",
      "status": "healthy",
      "requests": 350000,
      "errors": 8750,
      "avg_response_ms": 2.5
    },
    {
      "name": "FastCGI (PHP-FPM)",
      "backend_type": "fastcgi",
      "status": "healthy",
      "requests": 100000,
      "errors": 2500,
      "avg_response_ms": 5.0
    },
    {
      "name": "Static Files",
      "backend_type": "static",
      "status": "healthy",
      "requests": 91800,
      "errors": 0,
      "avg_response_ms": 0.1
    }
  ]
}
```

#### フィールド

**server**

| フィールド | 型 | 説明 |
|----------|-------|------|
| `version` | string | サーバーバージョン |
| `uptime_seconds` | integer | 稼働時間（秒） |
| `pid` | integer | プロセスID |
| `started_at` | integer | 起動時刻（Unix timestamp） |

**metrics**

| フィールド | 型 | 説明 |
|----------|-------|------|
| `requests_per_second` | float | 1秒あたりのリクエスト数 |
| `active_connections` | integer | アクティブ接続数 |
| `total_requests` | integer | 総リクエスト数 |
| `error_rate` | float | エラー率（パーセント） |

**backends[]**

| フィールド | 型 | 説明 |
|----------|-------|------|
| `name` | string | バックエンド名 |
| `backend_type` | string | バックエンドタイプ（`embedded`, `fastcgi`, `static`） |
| `status` | string | ステータス（`healthy`, `unhealthy`） |
| `requests` | integer | リクエスト数 |
| `errors` | integer | エラー数 |
| `avg_response_ms` | float | 平均応答時間（ミリ秒） |

---

### GET /api/health

ヘルスチェックエンドポイント。サーバーが正常に動作しているかを確認します。

#### リクエスト

```bash
# HTTP
curl http://localhost:9001/api/health

# Unix Socket
echo '{"command":"health"}' | socat - UNIX-CONNECT:/var/run/fe-php-admin.sock
echo "health" | socat - UNIX-CONNECT:/var/run/fe-php-admin.sock  # テキストプロトコル
```

#### レスポンス

```json
{
  "status": "healthy",
  "uptime_seconds": 3600,
  "version": "0.1.0"
}
```

#### フィールド

| フィールド | 型 | 説明 |
|----------|-------|------|
| `status` | string | ヘルス状態（`healthy`, `unhealthy`） |
| `uptime_seconds` | integer | 稼働時間（秒） |
| `version` | string | サーバーバージョン |

---

### GET /api/logs/recent

最近のリクエストログを取得します。

#### リクエスト

```bash
# HTTP
curl "http://localhost:9001/api/logs/recent?limit=10"

# Unix Socket
echo '{"command":"logs","limit":10}' | socat - UNIX-CONNECT:/var/run/fe-php-admin.sock
```

#### クエリパラメータ

| パラメータ | 型 | デフォルト | 説明 |
|----------|-------|----------|------|
| `limit` | integer | `100` | 取得する最大ログ数 |

#### レスポンス

```json
{
  "logs": [
    {
      "timestamp": "2025-11-18T12:34:56.789Z",
      "level": "info",
      "request_id": "550e8400-e29b-41d4-a716-446655440000",
      "method": "GET",
      "uri": "/api/users",
      "status": 200,
      "duration_ms": 2,
      "memory_peak_mb": 10.5,
      "opcache_hit": true,
      "worker_id": 3,
      "remote_addr": "192.168.1.100:54321",
      "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)...",
      "waf_triggered": false
    }
  ]
}
```

#### フィールド

| フィールド | 型 | 説明 |
|----------|-------|------|
| `timestamp` | string | タイムスタンプ（ISO 8601） |
| `level` | string | ログレベル（`info`, `warn`, `error`） |
| `request_id` | string | リクエストID（UUID） |
| `method` | string | HTTPメソッド |
| `uri` | string | リクエストURI |
| `status` | integer | HTTPステータスコード |
| `duration_ms` | integer | 処理時間（ミリ秒） |
| `memory_peak_mb` | float | ピークメモリ使用量（MB） |
| `opcache_hit` | boolean | OPcacheヒット |
| `worker_id` | integer | ワーカーID（nullの場合あり） |
| `remote_addr` | string | リモートアドレス |
| `user_agent` | string | User-Agent（nullの場合あり） |
| `waf_triggered` | boolean | WAFがトリガーされたか |

---

### GET /api/logs/analysis

ログの分析結果を取得します。

#### リクエスト

```bash
# HTTP
curl http://localhost:9001/api/logs/analysis

# Unix Socket
echo '{"command":"analysis"}' | socat - UNIX-CONNECT:/var/run/fe-php-admin.sock
```

#### レスポンス

```json
{
  "total_requests": 1000,
  "error_count": 25,
  "top_endpoints": [
    {
      "path": "/api/users",
      "count": 450,
      "avg_duration_ms": 2.5,
      "error_count": 10,
      "error_rate": 0.022
    },
    {
      "path": "/api/products",
      "count": 300,
      "avg_duration_ms": 5.0,
      "error_count": 5,
      "error_rate": 0.017
    }
  ],
  "slow_requests": [
    {
      "timestamp": "2025-11-18T12:34:56.789Z",
      "level": "info",
      "request_id": "550e8400-e29b-41d4-a716-446655440000",
      "method": "GET",
      "uri": "/api/reports",
      "status": 200,
      "duration_ms": 250,
      "memory_peak_mb": 0.0,
      "opcache_hit": false,
      "worker_id": null,
      "remote_addr": "192.168.1.100:54321",
      "user_agent": null,
      "waf_triggered": false
    }
  ],
  "suspicious_activity": [
    {
      "ip_address": "192.168.1.200",
      "event_type": "scan",
      "count": 15,
      "description": "15 404 errors (possible scanning)"
    },
    {
      "ip_address": "192.168.1.201",
      "event_type": "errors",
      "count": 8,
      "description": "8 server errors"
    }
  ]
}
```

#### フィールド

**top_endpoints[]**

| フィールド | 型 | 説明 |
|----------|-------|------|
| `path` | string | エンドポイントパス |
| `count` | integer | リクエスト数 |
| `avg_duration_ms` | float | 平均処理時間（ミリ秒） |
| `error_count` | integer | エラー数 |
| `error_rate` | float | エラー率（0.0-1.0） |

**slow_requests[]**

処理時間が100ms以上のリクエスト（上位10件）。フィールドは`/api/logs/recent`と同じ。

**suspicious_activity[]**

| フィールド | 型 | 説明 |
|----------|-------|------|
| `ip_address` | string | IPアドレス |
| `event_type` | string | イベントタイプ（`scan`, `errors`） |
| `count` | integer | 回数 |
| `description` | string | 説明 |

検出ルール:
- `scan`: 同一IPから10回以上の404エラー
- `errors`: 同一IPから5回以上の5xxエラー

---

### GET /api/security/blocked-ips

ブロック済みIPアドレスの一覧を取得します。

#### リクエスト

```bash
# HTTP
curl http://localhost:9001/api/security/blocked-ips

# Unix Socket
echo '{"command":"blocked_ips"}' | socat - UNIX-CONNECT:/var/run/fe-php-admin.sock
```

#### レスポンス

```json
{
  "blocked_ips": [
    "192.168.1.200",
    "10.0.0.50",
    "203.0.113.100"
  ]
}
```

---

### POST /api/config/reload

設定ファイルをリロードします。サーバーを再起動せずに設定を反映できます。

#### リクエスト

```bash
# HTTP
curl -X POST http://localhost:9001/api/config/reload

# Unix Socket
echo '{"command":"reload_config"}' | socat - UNIX-CONNECT:/var/run/fe-php-admin.sock
```

#### レスポンス

成功時:
```json
{
  "status": "success",
  "message": "Configuration reloaded successfully"
}
```

失敗時:
```json
{
  "status": "error",
  "message": "Failed to reload configuration: Invalid TOML syntax"
}
```

#### 注意事項

以下の設定項目は再起動が必要です：
- `[server]` セクション（ポート、ホスト、ワーカー数）
- `[php]` セクション（libphpパス、ワーカープールサイズ）
- `[tls]` セクション

以下の設定項目はリロードで反映されます：
- `[backend.routing_rules]`（ルーティングルール）
- `[waf]`（WAFルール）
- `[logging]`（ログレベル）

---

### POST /api/workers/restart

PHPワーカープロセスを再起動します。メモリリークが疑われる場合に有用です。

#### リクエスト

```bash
# HTTP
curl -X POST http://localhost:9001/api/workers/restart

# Unix Socket
echo '{"command":"restart_workers"}' | socat - UNIX-CONNECT:/var/run/fe-php-admin.sock
```

#### レスポンス

成功時:
```json
{
  "status": "success",
  "message": "Workers restarted successfully"
}
```

失敗時:
```json
{
  "status": "error",
  "message": "Failed to restart workers: ..."
}
```

#### 動作

1. 新しいワーカーを起動
2. 既存のワーカーへの新規リクエストを停止
3. 既存のワーカーの処理中リクエストが完了するまで待機
4. 既存のワーカーを終了
5. 新しいワーカーでリクエストを処理開始

ダウンタイムなしで再起動が行われます。

---

### POST /api/security/block-ip

IPアドレスをブロックします。ブロックされたIPからのリクエストは403 Forbiddenで拒否されます。

#### リクエスト

```bash
# HTTP
curl -X POST http://localhost:9001/api/security/block-ip \
  -H "Content-Type: application/json" \
  -d '{"ip":"192.168.1.100"}'

# Unix Socket
echo '{"command":"block_ip","ip":"192.168.1.100"}' | \
  socat - UNIX-CONNECT:/var/run/fe-php-admin.sock
```

#### リクエストボディ

```json
{
  "ip": "192.168.1.100"
}
```

| フィールド | 型 | 必須 | 説明 |
|----------|-------|------|------|
| `ip` | string | はい | ブロックするIPアドレス |

#### レスポンス

成功時:
```json
{
  "status": "success",
  "message": "IP 192.168.1.100 blocked successfully"
}
```

失敗時:
```json
{
  "status": "error",
  "message": "Invalid IP address"
}
```

---

### POST /api/security/unblock-ip

ブロック済みのIPアドレスを解除します。

#### リクエスト

```bash
# HTTP
curl -X POST http://localhost:9001/api/security/unblock-ip \
  -H "Content-Type: application/json" \
  -d '{"ip":"192.168.1.100"}'

# Unix Socket
echo '{"command":"unblock_ip","ip":"192.168.1.100"}' | \
  socat - UNIX-CONNECT:/var/run/fe-php-admin.sock
```

#### リクエストボディ

```json
{
  "ip": "192.168.1.100"
}
```

| フィールド | 型 | 必須 | 説明 |
|----------|-------|------|------|
| `ip` | string | はい | ブロック解除するIPアドレス |

#### レスポンス

成功時:
```json
{
  "status": "success",
  "message": "IP 192.168.1.100 unblocked successfully"
}
```

失敗時:
```json
{
  "status": "error",
  "message": "IP not found in block list"
}
```

---

## エラーレスポンス

すべてのエンドポイントは、エラー時に以下の形式でレスポンスを返します。

### HTTPステータスコード

| コード | 説明 |
|-------|------|
| 200 | 成功 |
| 400 | リクエストエラー（無効なパラメータなど） |
| 403 | アクセス拒否（IP制限など） |
| 404 | エンドポイントが見つからない |
| 500 | サーバー内部エラー |

### エラーレスポンス例

```json
{
  "status": "error",
  "message": "Invalid parameter: limit must be between 1 and 1000",
  "code": "INVALID_PARAMETER"
}
```

| フィールド | 型 | 説明 |
|----------|-------|------|
| `status` | string | `"error"` 固定 |
| `message` | string | エラーメッセージ |
| `code` | string | エラーコード（オプション） |

### エラーコード一覧

| コード | 説明 |
|-------|------|
| `INVALID_PARAMETER` | パラメータが無効 |
| `COMMAND_NOT_AVAILABLE` | コマンドが利用できない（機能が無効化されている） |
| `COMMAND_FAILED` | コマンドの実行に失敗 |
| `UNAUTHORIZED` | 認証エラー |
| `FORBIDDEN` | アクセス拒否 |
| `NOT_FOUND` | リソースが見つからない |
| `INTERNAL_ERROR` | サーバー内部エラー |

---

## 使用例

### Bashスクリプトでの使用

```bash
#!/bin/bash
# monitor.sh - サーバー状態を監視してアラート

SOCKET="/var/run/fe-php-admin.sock"
ALERT_EMAIL="admin@example.com"

# 状態取得
STATUS=$(echo '{"command":"status"}' | socat - UNIX-CONNECT:$SOCKET)

# エラー率チェック
ERROR_RATE=$(echo "$STATUS" | jq -r '.metrics.error_rate')
if (( $(echo "$ERROR_RATE > 5.0" | bc -l) )); then
  echo "High error rate: $ERROR_RATE%" | mail -s "Alert: High Error Rate" $ALERT_EMAIL
fi

# アクティブ接続数チェック
CONNECTIONS=$(echo "$STATUS" | jq -r '.metrics.active_connections')
if [ "$CONNECTIONS" -gt 1000 ]; then
  echo "Too many connections: $CONNECTIONS" | mail -s "Alert: High Connections" $ALERT_EMAIL
fi
```

### Pythonでの使用

```python
#!/usr/bin/env python3
import socket
import json

def admin_api_call(command, **params):
    """Unix Socket経由でAdmin APIを呼び出す"""
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.connect("/var/run/fe-php-admin.sock")

    request = {"command": command, **params}
    sock.sendall(json.dumps(request).encode() + b'\n')

    response = b""
    while True:
        chunk = sock.recv(4096)
        if not chunk:
            break
        response += chunk

    sock.close()
    return json.loads(response.decode())

# 状態取得
status = admin_api_call("status")
print(f"Total requests: {status['metrics']['total_requests']}")
print(f"Error rate: {status['metrics']['error_rate']}%")

# IPブロック
result = admin_api_call("block_ip", ip="192.168.1.100")
print(result['message'])

# ブロック済みIP一覧
blocked = admin_api_call("blocked_ips")
print(f"Blocked IPs: {', '.join(blocked['blocked_ips'])}")
```

### curlでの使用（HTTP API）

```bash
# 状態取得
curl -s http://localhost:9001/api/status | jq .

# 最近のログ
curl -s "http://localhost:9001/api/logs/recent?limit=10" | jq '.logs[] | {uri, status, duration_ms}'

# ログ分析
curl -s http://localhost:9001/api/logs/analysis | jq '.suspicious_activity'

# IPブロック
curl -X POST http://localhost:9001/api/security/block-ip \
  -H "Content-Type: application/json" \
  -d '{"ip":"192.168.1.100"}'

# 設定リロード
curl -X POST http://localhost:9001/api/config/reload
```

### Prometheus連携

Prometheusでメトリクスを収集し、Alertmanagerでアラートを送信：

`prometheus.yml`:
```yaml
scrape_configs:
  - job_name: 'fe-php'
    static_configs:
      - targets: ['localhost:9090']
    metrics_path: '/_metrics'
    scrape_interval: 15s
```

`alert.rules`:
```yaml
groups:
  - name: fe-php
    rules:
      - alert: HighErrorRate
        expr: rate(backend_errors_total[5m]) / rate(backend_requests_total[5m]) > 0.05
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High error rate"
          description: "Error rate is {{ $value | humanizePercentage }}"
```

### Grafanaダッシュボード

Grafanaで可視化するクエリ例：

```promql
# リクエストレート（バックエンド別）
rate(backend_requests_total[5m])

# エラー率
rate(backend_errors_total[5m]) / rate(backend_requests_total[5m])

# P99レイテンシ
histogram_quantile(0.99, rate(backend_request_duration_seconds_bucket[5m]))

# アクティブ接続数
active_connections
```
