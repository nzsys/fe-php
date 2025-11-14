# fe-php 機能一覧

本ドキュメントでは、fe-phpの全機能について詳細に説明します。

## 目次

1. [ハイブリッドバックエンド](#ハイブリッドバックエンド)
2. [ネットワーク機能](#ネットワーク機能)
3. [セキュリティ機能](#セキュリティ機能)
4. [パフォーマンス最適化](#パフォーマンス最適化)
5. [可観測性](#可観測性)
6. [運用機能](#運用機能)

---

## ハイブリッドバックエンド

### 1. Embeddedバックエンド (libphp直接実行)

**説明**: libphpをプロセス内に組み込み、PHPスクリプトを直接実行します。

**利点**:
- **最高速**: プロセス間通信なし、関数呼び出しのみ
- **詳細メトリクス**: メモリ使用量、OPcache統計を直接取得
- **完全制御**: PHP設定を動的に変更可能

**設定例**:
```toml
[backend]
default_backend = "embedded"

[[backend.routing_rules]]
pattern = { prefix = "/api/" }
backend = "embedded"
priority = 100
```

**使用例**:
```bash
# APIエンドポイントは自動的にembeddedバックエンドで処理
curl http://localhost:8080/api/users
```

### 2. FastCGIバックエンド (PHP-FPMプロキシ)

**説明**: PHP-FPMへのリバースプロキシとして動作します。

**利点**:
- **高安定性**: PHPクラッシュがサーバー全体に影響しない
- **既存資産活用**: 既存のPHP-FPM設定をそのまま使用
- **柔軟な構成**: 複数のPHP-FPMプールに分散可能

**接続方式**:
- **TCP接続**: `127.0.0.1:9000`
- **Unixソケット**: `unix:/var/run/php-fpm.sock`

**設定例**:
```toml
[php]
fastcgi_address = "unix:/var/run/php-fpm.sock"
# fastcgi_address = "127.0.0.1:9000"

[php.connection_pool]
max_size = 20
max_idle_time_secs = 60
max_lifetime_secs = 3600
```

### 3. Staticバックエンド (静的ファイル配信)

**説明**: 静的ファイルを高速に配信する専用バックエンド。

**対応ファイル**:
- 画像: `.jpg`, `.png`, `.gif`, `.webp`, `.svg`
- CSS/JS: `.css`, `.js`, `.mjs`
- フォント: `.woff`, `.woff2`, `.ttf`, `.eot`
- その他: `.html`, `.json`, `.xml`, `.pdf`

**設定例**:
```toml
[backend.static_files]
document_root = "/var/www/html/public"
index_files = ["index.html", "index.htm"]
enable_etag = true
enable_range_request = true

# 画像は静的ファイル配信
[[backend.routing_rules]]
pattern = { suffix = ".jpg" }
backend = "static"
priority = 90
```

### 4. パターンベースルーティング

**パターンタイプ**:

#### Exact (完全一致)
```toml
[[backend.routing_rules]]
pattern = { exact = "/health" }
backend = "static"
priority = 100
```

#### Prefix (プレフィックス一致)
```toml
[[backend.routing_rules]]
pattern = { prefix = "/api/" }
backend = "embedded"
priority = 90
```

#### Suffix (サフィックス一致)
```toml
[[backend.routing_rules]]
pattern = { suffix = ".jpg" }
backend = "static"
priority = 80
```

#### Regex (正規表現)
```toml
[[backend.routing_rules]]
pattern = { regex = "^/uploads/\\d+/.*\\.(jpg|png)$" }
backend = "static"
priority = 70
```

---

## ネットワーク機能

### 1. Unix Socket サポート

**説明**: TCPソケットに加えてUnix socketでのリスニングをサポート。Nginxなどのリバースプロキシと高速に通信できます。

**利点**:
- **高速化**: TCP/IPのオーバーヘッドがなくなり、ローカル通信が高速化（約10-15%のレイテンシ削減）
- **セキュリティ**: ネットワーク経由のアクセスを防ぎ、ファイルシステムの権限でアクセス制御
- **リソース効率**: ポート番号を消費しない

**設定方法**:
```toml
[server]
listen_type = "unix"  # "tcp" または "unix"
unix_socket_path = "/tmp/fe-php.sock"
workers = 4
enable_http2 = true
```

**Nginxとの連携例**:
```nginx
upstream fe-php {
    server unix:/tmp/fe-php.sock;
}

server {
    listen 80;
    server_name example.com;

    location / {
        proxy_pass http://fe-php;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
    }
}
```

**トラブルシューティング**:

Unix socketファイルが作成されない場合:
```bash
# ディレクトリの書き込み権限を確認
ls -la /tmp/

# 既存のソケットファイルを削除
rm /tmp/fe-php.sock
```

Permission denied エラーの場合:
```bash
# ソケットファイルの権限を変更
chmod 666 /tmp/fe-php.sock

# または、グループ権限を設定
chgrp www-data /tmp/fe-php.sock
chmod 660 /tmp/fe-php.sock
```

### 2. HTTP/2 サポート

**説明**: HTTP/2プロトコルに対応し、多重化通信とヘッダー圧縮を実現します。

**利点**:
- **多重化**: 1つのTCP接続で複数のリクエストを同時処理
- **ヘッダー圧縮**: HPACKによる帯域幅削減（約30-40%）
- **サーバープッシュ**: レイテンシの削減
- **パフォーマンス向上**: 多数の小リクエストで約20-30%のレイテンシ削減

**設定方法**:
```toml
[server]
enable_http2 = true
```

**動作確認**:
```bash
# HTTP/2でリクエスト
curl --http2 http://localhost:8080/test.php -v

# ログで確認
# HTTP/2 support enabled
```

**注意事項**:
- ブラウザからのアクセスにはTLS/HTTPSが必要な場合があります
- `curl`の`--http2`オプションでテスト可能

### 3. TLS/SSL対応

**機能**:
- TLS 1.2, 1.3対応
- SNI (Server Name Indication)対応
- Let's Encrypt証明書対応
- 自動HTTP→HTTPS リダイレクト

**設定例**:
```toml
[tls]
cert_path = "/etc/letsencrypt/live/example.com/fullchain.pem"
key_path = "/etc/letsencrypt/live/example.com/privkey.pem"
http_redirect = true    # HTTPを自動的にHTTPSへリダイレクト
http_port = 80
https_port = 443
```

### 4. HTTP→HTTPS自動リダイレクト

**機能**: HTTPリクエストを自動的にHTTPSへリダイレクト（301 Moved Permanently）

**設定**:
```toml
[tls]
http_redirect = true
http_port = 80          # HTTPリスニングポート
```

**動作例**:
```bash
# HTTPアクセス
curl -I http://example.com/api/users

# 自動リダイレクト
HTTP/1.1 301 Moved Permanently
Location: https://example.com/api/users
```

### 5. Keep-Alive接続

**機能**: HTTP Keep-Alive対応で接続を再利用

**設定例**:
```toml
[server]
keep_alive_timeout_secs = 60
max_requests_per_connection = 1000
```

### 6. Range Request (部分コンテンツ配信)

**機能**: HTTP 206 Partial Content対応、動画/音声ストリーミングに最適

**対応パターン**:
```http
Range: bytes=0-1023          # 先頭1024バイト
Range: bytes=1000-           # 1000バイト目から最後まで
Range: bytes=-500            # 最後の500バイト
```

**レスポンス例**:
```http
HTTP/1.1 206 Partial Content
Content-Range: bytes 0-1023/5000
Content-Length: 1024
Accept-Ranges: bytes
```

---

## セキュリティ機能

### 1. WAF (Web Application Firewall)

**検知パターン**:

#### SQLインジェクション
```
SELECT * FROM users WHERE id=1 OR 1=1
UNION SELECT password FROM users
```

#### XSS (Cross-Site Scripting)
```html
<script>alert('XSS')</script>
<img src=x onerror=alert(1)>
```

#### パストラバーサル
```
../../etc/passwd
..%2F..%2Fetc%2Fpasswd
```

**動作モード**:
```toml
[waf]
mode = "block"   # "off", "learn", "detect", "block"
log_blocked = true
```

| モード | 説明 |
|--------|------|
| `off` | WAF無効 |
| `learn` | パターン学習（ブロックしない） |
| `detect` | 検知のみログ出力 |
| `block` | 検知してブロック |

### 2. IPフィルタリング (CIDR対応)

**ホワイトリストモード**:
```toml
[security]
ip_filter_mode = "whitelist"
allowed_ips = [
    "192.168.1.0/24",    # ローカルネットワーク
    "10.0.0.0/8",        # 社内ネットワーク
    "203.0.113.5/32",    # 特定IP
]
```

**ブラックリストモード**:
```toml
[security]
ip_filter_mode = "blacklist"
denied_ips = [
    "203.0.113.0/24",    # 攻撃元ネットワーク
]
```

### 3. CORS (Cross-Origin Resource Sharing)

**基本設定**:
```toml
[cors]
allowed_origins = ["https://example.com", "https://app.example.com"]
allowed_methods = ["GET", "POST", "PUT", "DELETE", "OPTIONS"]
allowed_headers = ["Content-Type", "Authorization", "X-Requested-With"]
exposed_headers = ["X-Total-Count"]
max_age = 3600
allow_credentials = true
```

**プリフライトリクエスト対応**:
```http
OPTIONS /api/users HTTP/1.1
Origin: https://example.com

HTTP/1.1 204 No Content
Access-Control-Allow-Origin: https://example.com
Access-Control-Allow-Methods: GET, POST, PUT, DELETE
Access-Control-Max-Age: 3600
```

### 4. レート制限

**IP別レート制限**:
```toml
[rate_limit]
enabled = true
requests_per_second = 100
burst_size = 200
```

**動作例**:
```bash
# 101リクエスト目
HTTP/1.1 429 Too Many Requests
Retry-After: 1
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1699999999
```

### 5. GeoIP制限

**国別アクセス制御**:
```toml
[geoip]
database_path = "/usr/share/GeoIP/GeoLite2-Country.mmdb"
allowed_countries = ["JP", "US", "GB"]  # 日本、米国、英国のみ許可
denied_countries = ["CN", "RU"]         # 中国、ロシアを拒否
```

---

## パフォーマンス最適化

### 1. 接続プーリング

**FastCGI接続プール**:
```toml
[backend.connection_pool]
max_size = 20                    # 最大プールサイズ
max_idle_time_secs = 60          # アイドルタイムアウト
max_lifetime_secs = 3600         # 接続最大寿命
connect_timeout_secs = 5         # 接続タイムアウト
enable_metrics = true            # メトリクス収集を有効化
```

**効果**:
- 接続確立オーバーヘッド削減
- レイテンシ改善（約30%高速化）
- リソース効率の向上

**メトリクスの確認方法**:
```bash
curl http://localhost:9090/_metrics | grep connection_pool
```

### 2. Circuit Breaker

**説明**: バックエンドが不安定な場合に自動的に接続を遮断し、カスケード障害を防ぎます。

**設定方法**:
```toml
[backend.connection_pool.circuit_breaker]
enable = true                    # Circuit breakerを有効化
failure_threshold = 5            # この回数失敗したら回路を開く
success_threshold = 2            # この回数成功したら回路を閉じる
timeout_seconds = 60             # 回路を開いたまま保持する時間
half_open_max_requests = 3       # Half-open状態での最大リクエスト数
```

**動作の仕組み**:
1. **Closed状態**: 正常動作中
2. **Open状態**: `failure_threshold`回失敗後、全てのリクエストを拒否
3. **Half-open状態**: `timeout_seconds`経過後、`half_open_max_requests`個のリクエストで試行
4. **Closed状態**: `success_threshold`回成功したら復帰

**利点**:
- **カスケード障害の防止**: 不安定なバックエンドへの過剰な接続を防ぐ
- **自動復旧**: バックエンドが回復したら自動的に接続を再開
- **リソース保護**: 失敗するリクエストにリソースを浪費しない

**トラブルシューティング**:

Circuit breakerが開きっぱなしの場合:
- `timeout_seconds`を短くする
- `failure_threshold`を増やす
- バックエンドの健全性を確認

### 3. 静的ファイル圧縮 (gzip/brotli)

**自動圧縮**:
```toml
[compression]
enable_gzip = true
enable_brotli = true
min_size = 1024              # 1KB未満は圧縮しない
gzip_level = 6               # 1-9 (デフォルト6)
brotli_quality = 6           # 0-11 (デフォルト6)

# 圧縮対象Content-Type
compressible_types = [
    "text/html",
    "text/css",
    "text/javascript",
    "application/json",
    "application/xml",
]
```

**動作**:
```http
GET /app.js HTTP/1.1
Accept-Encoding: br, gzip

HTTP/1.1 200 OK
Content-Encoding: br         # Brotli優先
Content-Length: 12345        # 圧縮後サイズ
```

**圧縮率**:
| ファイル | 元サイズ | gzip | brotli |
|---------|---------|------|--------|
| HTML | 100KB | 20KB (80%) | 18KB (82%) |
| CSS | 50KB | 10KB (80%) | 9KB (82%) |
| JSON | 200KB | 40KB (80%) | 35KB (82.5%) |

### 4. ETagキャッシング

**ETag生成**:
```
ETag = SHA256(mtime + file_size)
```

**動作例**:
```http
# 初回リクエスト
GET /image.jpg HTTP/1.1

HTTP/1.1 200 OK
ETag: "abc123def456"
Last-Modified: Wed, 14 Nov 2025 12:00:00 GMT
Cache-Control: public, max-age=31536000

# 2回目（変更なし）
GET /image.jpg HTTP/1.1
If-None-Match: "abc123def456"

HTTP/1.1 304 Not Modified
```

### 5. OPcache最適化

**設定**:
```toml
[php]
opcache_enable = true
opcache_memory = "128M"
opcache_max_files = 10000
opcache_validate_timestamps = false  # 本番環境ではfalse推奨
opcache_jit = "tracing"              # JIT有効化 (PHP 8.0+)
```

---

## 可観測性

### 1. Prometheusメトリクス

**エンドポイント**: `GET /_metrics`

**メトリクス例**:
```prometheus
# HTTPリクエスト
http_requests_total{method="GET",status="200"} 12345
http_request_duration_seconds{method="GET"} 0.005

# バックエンド別
backend_requests_total{backend="embedded",status="success"} 8000
backend_requests_total{backend="fastcgi",status="success"} 3000
backend_requests_total{backend="static",status="success"} 1345

backend_request_duration_seconds{backend="embedded"} 0.003
backend_request_duration_seconds{backend="fastcgi"} 0.008
backend_request_duration_seconds{backend="static"} 0.001

# エラー
backend_errors_total{backend="embedded",error_type="timeout"} 5
backend_errors_total{backend="fastcgi",error_type="connection_failed"} 2

# PHP
php_workers{status="idle"} 8
php_workers{status="busy"} 2
php_memory_bytes{worker_id="0"} 67108864
opcache_hit_rate_percent 95.5
opcache_memory_bytes 134217728
opcache_cached_scripts 1250

# 接続プール
connection_pool_idle_connections{backend="fastcgi",pool_type="tcp"} 15
connection_pool_active_connections{backend="fastcgi",pool_type="tcp"} 5
connection_pool_acquire_duration_seconds{backend="fastcgi"} 0.001
connection_pool_errors_total{backend="fastcgi",pool_type="tcp",error_type="timeout"} 2

# Circuit Breaker
circuit_breaker_state{backend="fastcgi"} 0  # 0=Closed, 1=Open, 2=HalfOpen
circuit_breaker_failures_total{backend="fastcgi"} 3
```

### 2. 構造化ログ (JSON)

**設定**:
```toml
[logging]
level = "info"
format = "json"
output = "/var/log/fe-php/app.log"
```

**出力例**:
```json
{
  "timestamp": "2025-11-14T12:00:00.123Z",
  "level": "INFO",
  "target": "fe_php::server",
  "fields": {
    "message": "Request completed",
    "method": "GET",
    "path": "/api/users",
    "status": 200,
    "duration_ms": 5,
    "backend": "embedded",
    "request_id": "req_abc123",
    "remote_addr": "192.168.1.100"
  }
}
```

### 3. OpenTelemetry対応

**分散トレーシング**:
```toml
[tracing]
enabled = true
endpoint = "http://localhost:4317"  # OTLP gRPC
service_name = "fe-php"
```

**トレースデータ**:
- スパンID、トレースID
- 親子関係
- バックエンド実行時間
- 外部APIコール

### 4. ヘルスチェック

**エンドポイント**: `GET /health`

**レスポンス**:
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_secs": 86400,
  "backends": {
    "embedded": "healthy",
    "fastcgi": "healthy",
    "static": "healthy"
  },
  "metrics": {
    "active_connections": 45,
    "total_requests": 1234567,
    "error_rate": 0.0012
  }
}
```

---

## 運用機能

### 1. Admin Console (管理コンソール)

**説明**: Webベースの管理インターフェース

**アクセス方法**: `http://localhost:9000/` (デフォルト)

**機能**:
- **Dashboard (/)**: リアルタイムメトリクスとバックエンド状態を表示
  - サーバー情報: バージョン、稼働時間、PID、起動日時
  - メトリクス: RPS、アクティブ接続数、総リクエスト数、エラー率
  - バックエンド状態: 各バックエンドの状態・リクエスト数・エラー数・平均応答時間
- **Metrics (/metrics)**: 詳細メトリクスビューア
- **Logs (/logs)**: リアルタイムログビューア（フィルタリング・検索機能付き）
- **WAF (/waf)**: WAF統計とブロック済みIP表示
- **Backends (/backends)**: バックエンド別の詳細情報
- **System (/system)**: システムリソース情報（CPU、メモリ、プロセス数）
- **JSON API (/api/status)**: プログラマティックアクセス用のJSON形式ステータス

**セキュリティ**:
- デフォルトで `127.0.0.1` (localhost) にバインド
- `allowed_ips` によるIPアドレス制限対応
- 読み取り専用インターフェース（設定変更不可）

**設定例**:
```toml
[admin]
enable = true
host = "127.0.0.1"      # localhostのみアクセス可能
http_port = 9000
allowed_ips = ["127.0.0.1"]
```

**APIレスポンス例**:
```json
{
  "server": {
    "version": "0.1.0",
    "uptime_seconds": 3600,
    "pid": 12345,
    "started_at": 1731590400
  },
  "metrics": {
    "requests_per_second": 120.5,
    "active_connections": 45,
    "total_requests": 433800,
    "error_rate": 0.12
  },
  "backends": [
    {
      "name": "Embedded (libphp)",
      "backend_type": "embedded",
      "status": "healthy",
      "requests": 320000,
      "errors": 42,
      "avg_response_ms": 1.2
    }
  ]
}
```

### 2. グレースフルシャットダウン

**シグナル**:
- `SIGTERM`: 優雅なシャットダウン
- `SIGINT` (Ctrl+C): 優雅なシャットダウン

**動作**:
1. 新規接続受付停止
2. 既存リクエスト完了待機（最大30秒）
3. タイムアウト後、強制終了

**設定**:
```toml
[server]
shutdown_timeout_secs = 30
```

### 3. 設定ホットリロード

**シグナル**: `SIGUSR1`

**実行方法**:
```bash
# プロセスIDを取得
pgrep fe-php

# リロードシグナル送信
kill -USR1 $(pgrep fe-php)

# またはsystemdの場合
systemctl reload fe-php
```

**リロード可能項目**:
- ルーティングルール
- バックエンド設定
- ミドルウェア設定（WAF、CORS、レート制限）
- ログレベル

**リロード不可項目**:
- ポート番号
- TLS証明書（要再起動）
- ワーカー数

### 4. プロセス管理

**systemdユニットファイル例**:
```ini
[Unit]
Description=fe-php Application Server
After=network.target

[Service]
Type=simple
User=www-data
Group=www-data
ExecStart=/usr/local/bin/fe-php serve --config /etc/fe-php/config.toml
ExecReload=/bin/kill -USR1 $MAINPID
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

### 5. ログローテーション

**logrotateサンプル**:
```
/var/log/fe-php/*.log {
    daily
    rotate 14
    compress
    delaycompress
    notifempty
    create 0640 www-data www-data
    sharedscripts
    postrotate
        /bin/kill -USR1 $(cat /var/run/fe-php.pid) 2>/dev/null || true
    endscript
}
```

---

## パフォーマンス指標

### Unix Socket
- **レイテンシ**: TCP比で約10-15%削減
- **スループット**: 小さなリクエストで約5-10%向上
- **CPU使用率**: TCP/IPスタックを回避することで約3-5%削減

### HTTP/2
- **同時接続**: 複数リクエストを1接続で処理可能
- **ヘッダー圧縮**: 帯域幅を約30-40%削減
- **レイテンシ**: 多数の小リクエストで約20-30%削減

### Connection Pooling + Circuit Breaker
- **安定性**: カスケード障害の防止
- **レスポンス時間**: 失敗時の待機時間を削減
- **リソース効率**: 不要な接続試行を回避
