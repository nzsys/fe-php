# Configuration

fe-phpの設定ファイル（TOML形式）の詳細なリファレンスです。

## 基本構造

設定ファイルは以下のセクションで構成されます：

```toml
[server]        # HTTPサーバー設定
[php]           # PHP実行環境設定
[backend]       # バックエンドルーティング設定
[admin]         # Admin API設定
[metrics]       # メトリクス設定
[logging]       # ログ設定
[waf]           # WAF設定
[tls]           # TLS/SSL設定
[geoip]         # GeoIPフィルタリング設定
[redis]         # Redis統合設定
[tracing]       # OpenTelemetry分散トレーシング設定
[load_balancing]  # ロードバランシング設定
[deployment]    # デプロイメント戦略設定
```

## [server]

HTTPサーバーの基本設定。

```toml
[server]
host = "0.0.0.0"
port = 8080
workers = 4
enable_http2 = false
multi_process = false
process_count = 4
listen_type = "tcp"
# unix_socket_path = "/var/run/fe-php.sock"
```

### パラメータ

| パラメータ | 型 | デフォルト | 説明 |
|----------|-------|----------|------|
| `host` | string | `"0.0.0.0"` | バインドするホスト |
| `port` | integer | `8080` | バインドするポート |
| `workers` | integer | CPU数 | ワーカースレッド数 |
| `enable_http2` | boolean | `false` | HTTP/2サポートを有効化 |
| `multi_process` | boolean | `false` | マルチプロセスモード |
| `process_count` | integer | `4` | マルチプロセス時のプロセス数 |
| `listen_type` | string | `"tcp"` | リスナータイプ（`tcp` または `unix`） |
| `unix_socket_path` | string | - | Unix Socketパス（`listen_type = "unix"`時） |

### 推奨設定

- **開発環境**: `workers = 2-4`
- **本番環境**: `workers = CPU数 × 1.5`
- **高負荷環境**: `workers = CPU数 × 2`

## [php]

PHP実行環境の設定。

```toml
[php]
libphp_path = "/usr/local/php-zts-embed/lib/libphp.so"
document_root = "/var/www/html"
worker_pool_size = 8
worker_max_requests = 10000
use_fpm = false
fpm_socket = "127.0.0.1:9000"

[php.opcache]
enable = true
memory_size = "256M"
max_files = 10000
validate_timestamps = false
```

### パラメータ

| パラメータ | 型 | デフォルト | 説明 |
|----------|-------|----------|------|
| `libphp_path` | string | - | libphpの共有ライブラリパス（`.so`または`.dylib`） |
| `document_root` | string | - | PHPファイルのルートディレクトリ |
| `worker_pool_size` | integer | `8` | PHPワーカープールサイズ |
| `worker_max_requests` | integer | `10000` | ワーカーの最大リクエスト処理数（メモリリーク対策） |
| `use_fpm` | boolean | `false` | PHP-FPMを使用するか |
| `fpm_socket` | string | `"127.0.0.1:9000"` | PHP-FPMのソケット（TCP: `host:port`、Unix: `/path/to/socket`） |

### [php.opcache]

| パラメータ | 型 | デフォルト | 説明 |
|----------|-------|----------|------|
| `enable` | boolean | `true` | OPcacheを有効化 |
| `memory_size` | string | `"256M"` | OPcacheメモリサイズ |
| `max_files` | integer | `10000` | キャッシュする最大ファイル数 |
| `validate_timestamps` | boolean | `false` | ファイルのタイムスタンプを検証（開発時は`true`、本番は`false`推奨） |

## [backend]

バックエンドルーティングの設定。

```toml
[backend]
enable_hybrid = true
default_backend = "embedded"

[[backend.routing_rules]]
pattern = { type = "prefix", value = "/api/" }
backend = "embedded"
priority = 100

[[backend.routing_rules]]
pattern = { type = "suffix", value = ".jpg" }
backend = "static"
priority = 90

[backend.static_files]
enable = true
root = "/var/www/html"
index_files = ["index.html", "index.htm"]

[backend.connection_pool]
max_size = 50
max_idle_time_secs = 300
max_lifetime_secs = 3600
connect_timeout_secs = 10
enable_metrics = true

[backend.connection_pool.circuit_breaker]
enable = true
failure_threshold = 10
success_threshold = 3
timeout_seconds = 30
half_open_max_requests = 5
```

### パラメータ

| パラメータ | 型 | デフォルト | 説明 |
|----------|-------|----------|------|
| `enable_hybrid` | boolean | `false` | ハイブリッドバックエンドを有効化 |
| `default_backend` | string | `"embedded"` | デフォルトバックエンド（`embedded`, `fastcgi`, `static`） |

### [[backend.routing_rules]]

リクエストをバックエンドにルーティングするルール。複数定義可能。優先度の高い順に評価されます。

```toml
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/api/" }
backend = "embedded"
priority = 100
```

#### パターンタイプ

| タイプ | 説明 | 例 |
|-------|------|-----|
| `prefix` | URIがプレフィックスと一致 | `{ type = "prefix", value = "/api/" }` |
| `suffix` | URIがサフィックスと一致 | `{ type = "suffix", value = ".jpg" }` |
| `exact` | URIが完全一致 | `{ type = "exact", value = "/status" }` |
| `regex` | URIが正規表現と一致 | `{ type = "regex", value = "^/user/[0-9]+$" }` |

#### バックエンド

| バックエンド | 説明 |
|-----------|------|
| `embedded` | libphpを直接実行。最高速だがメモリを共有 |
| `fastcgi` | PHP-FPMへプロキシ。プロセス分離で安定 |
| `static` | Rustで直接ファイル配信。PHPオーバーヘッドなし |

### [backend.static_files]

静的ファイルバックエンドの設定。

| パラメータ | 型 | デフォルト | 説明 |
|----------|-------|----------|------|
| `enable` | boolean | `false` | 静的ファイルバックエンドを有効化 |
| `root` | string | - | 静的ファイルのルートディレクトリ |
| `index_files` | array | `["index.html"]` | ディレクトリリクエスト時のインデックスファイル |

### [backend.connection_pool]

FastCGI接続プールの設定。

| パラメータ | 型 | デフォルト | 説明 |
|----------|-------|----------|------|
| `max_size` | integer | `50` | プール内の最大接続数 |
| `max_idle_time_secs` | integer | `300` | アイドル接続の最大保持時間（秒） |
| `max_lifetime_secs` | integer | `3600` | 接続の最大生存時間（秒） |
| `connect_timeout_secs` | integer | `10` | 接続タイムアウト（秒） |
| `enable_metrics` | boolean | `true` | 接続プールメトリクスを有効化 |

### [backend.connection_pool.circuit_breaker]

接続プールのサーキットブレーカー設定。

| パラメータ | 型 | デフォルト | 説明 |
|----------|-------|----------|------|
| `enable` | boolean | `false` | サーキットブレーカーを有効化 |
| `failure_threshold` | integer | `10` | サーキットを開く失敗回数 |
| `success_threshold` | integer | `3` | サーキットを閉じる成功回数 |
| `timeout_seconds` | integer | `30` | ハーフオープン状態に移行するまでの時間（秒） |
| `half_open_max_requests` | integer | `5` | ハーフオープン状態で許可する最大リクエスト数 |

## [admin]

Admin APIの設定。

```toml
[admin]
enable = true
host = "127.0.0.1"
unix_socket = "/var/run/fe-php-admin.sock"
http_port = 9001
allowed_ips = ["127.0.0.1", "::1"]
```

### パラメータ

| パラメータ | 型 | デフォルト | 説明 |
|----------|-------|----------|------|
| `enable` | boolean | `false` | Admin APIを有効化 |
| `host` | string | `"127.0.0.1"` | HTTP APIのバインドホスト |
| `unix_socket` | string | - | Unix Socketパス |
| `http_port` | integer | `9001` | HTTP APIのポート |
| `allowed_ips` | array | `[]` | HTTP APIへのアクセスを許可するIP（CIDR表記可） |

## [metrics]

Prometheusメトリクスの設定。

```toml
[metrics]
enable = true
endpoint = "/_metrics"
port = 9090
```

### パラメータ

| パラメータ | 型 | デフォルト | 説明 |
|----------|-------|----------|------|
| `enable` | boolean | `false` | メトリクスエンドポイントを有効化 |
| `endpoint` | string | `"/_metrics"` | メトリクスエンドポイントのパス |
| `port` | integer | `9090` | メトリクスサーバーのポート |

## [logging]

ログの設定。

```toml
[logging]
level = "info"
format = "json"
output = "stdout"
```

### パラメータ

| パラメータ | 型 | デフォルト | 説明 |
|----------|-------|----------|------|
| `level` | string | `"info"` | ログレベル（`trace`, `debug`, `info`, `warn`, `error`） |
| `format` | string | `"json"` | ログ形式（`json`, `text`） |
| `output` | string | `"stdout"` | ログ出力先（`stdout`, `stderr`, またはファイルパス） |

## [waf]

Web Application Firewallの設定。

```toml
[waf]
enable = true
mode = "block"
rules_path = "waf_rules.toml"

[waf.rate_limit]
requests_per_ip = 100
window_seconds = 60
burst = 20
```

### パラメータ

| パラメータ | 型 | デフォルト | 説明 |
|----------|-------|----------|------|
| `enable` | boolean | `false` | WAFを有効化 |
| `mode` | string | `"detect"` | 動作モード（`detect`: 検出のみ、`block`: ブロック） |
| `rules_path` | string | - | WAFルールファイルのパス |

### [waf.rate_limit]

| パラメータ | 型 | デフォルト | 説明 |
|----------|-------|----------|------|
| `requests_per_ip` | integer | `100` | IP別の最大リクエスト数 |
| `window_seconds` | integer | `60` | レート制限のウィンドウサイズ（秒） |
| `burst` | integer | `20` | 一時的に許可する最大バースト |

## [tls]

TLS/SSLの設定。

```toml
[tls]
enable = false
cert_path = "/etc/ssl/certs/server.crt"
key_path = "/etc/ssl/private/server.key"
ca_cert_path = "/etc/ssl/certs/ca.crt"
alpn_protocols = ["h2", "http/1.1"]
http_redirect = true
http_port = 80
```

### パラメータ

| パラメータ | 型 | デフォルト | 説明 |
|----------|-------|----------|------|
| `enable` | boolean | `false` | TLS/SSLを有効化 |
| `cert_path` | string | - | TLS証明書のパス |
| `key_path` | string | - | TLS秘密鍵のパス |
| `ca_cert_path` | string | - | CA証明書のパス（クライアント認証用） |
| `alpn_protocols` | array | `["h2", "http/1.1"]` | ALPNプロトコル |
| `http_redirect` | boolean | `false` | HTTPをHTTPSにリダイレクト |
| `http_port` | integer | `80` | リダイレクト元のHTTPポート |

## [geoip]

GeoIPフィルタリングの設定。

```toml
[geoip]
enable = false
database_path = "/usr/share/GeoIP/GeoLite2-Country.mmdb"
allowed_countries = ["JP", "US", "GB"]
blocked_countries = ["CN", "RU"]
```

### パラメータ

| パラメータ | 型 | デフォルト | 説明 |
|----------|-------|----------|------|
| `enable` | boolean | `false` | GeoIPフィルタリングを有効化 |
| `database_path` | string | - | MaxMind GeoIPデータベースのパス |
| `allowed_countries` | array | `[]` | 許可する国コード（ISO 3166-1 alpha-2） |
| `blocked_countries` | array | `[]` | ブロックする国コード（`blocked_countries`が優先） |

## [redis]

Redisセッション管理の設定。

```toml
[redis]
enable = false
url = "redis://127.0.0.1:6379"
pool_size = 20
timeout_ms = 5000
key_prefix = "fe_php:session:"
```

### パラメータ

| パラメータ | 型 | デフォルト | 説明 |
|----------|-------|----------|------|
| `enable` | boolean | `false` | Redis統合を有効化 |
| `url` | string | - | Redis接続URL |
| `pool_size` | integer | `20` | 接続プールサイズ |
| `timeout_ms` | integer | `5000` | 接続タイムアウト（ミリ秒） |
| `key_prefix` | string | `"fe_php:session:"` | セッションキーのプレフィックス |

## [tracing]

OpenTelemetry分散トレーシングの設定。

```toml
[tracing]
enable = false
otlp_endpoint = "http://localhost:4317"
service_name = "fe-php-production"
sample_rate = 0.1
```

### パラメータ

| パラメータ | 型 | デフォルト | 説明 |
|----------|-------|----------|------|
| `enable` | boolean | `false` | 分散トレーシングを有効化 |
| `otlp_endpoint` | string | - | OTLPエンドポイント |
| `service_name` | string | `"fe-php"` | トレーシングのサービス名 |
| `sample_rate` | float | `0.1` | サンプリングレート（0.0-1.0、1.0=全リクエスト） |

## [load_balancing]

ロードバランシングの設定。

```toml
[load_balancing]
enable = false
algorithm = "least_conn"

[[load_balancing.upstreams]]
name = "backend-1"
url = "http://192.168.1.10:8080"
weight = 3
enabled = true

[load_balancing.health_check]
enable = true
path = "/_health"
interval_seconds = 30
timeout_seconds = 5
unhealthy_threshold = 3
healthy_threshold = 2

[load_balancing.circuit_breaker]
enable = true
failure_threshold = 5
success_threshold = 2
timeout_seconds = 60
half_open_max_requests = 3
```

### パラメータ

| パラメータ | 型 | デフォルト | 説明 |
|----------|-------|----------|------|
| `enable` | boolean | `false` | ロードバランシングを有効化 |
| `algorithm` | string | `"round_robin"` | アルゴリズム（`round_robin`, `least_conn`, `weighted_round_robin`, `ip_hash`） |

### [[load_balancing.upstreams]]

| パラメータ | 型 | デフォルト | 説明 |
|----------|-------|----------|------|
| `name` | string | - | バックエンド名 |
| `url` | string | - | バックエンドURL |
| `weight` | integer | `1` | 重み（`weighted_round_robin`使用時） |
| `enabled` | boolean | `true` | バックエンドを有効化 |

## [deployment]

デプロイメント戦略（A/Bテスト、カナリーリリース）の設定。

```toml
[deployment]
enable = false
strategy = "canary"
sticky_sessions = true

[[deployment.variants]]
name = "stable"
weight = 90
upstream = "http://stable-backend:8080"
metrics_tracking = true

[[deployment.variants]]
name = "canary"
weight = 10
upstream = "http://canary-backend:8080"
metrics_tracking = true

[deployment.canary]
max_error_rate = 0.05
max_response_time_ms = 500
min_observation_period_secs = 300
min_requests_before_decision = 100
```

### パラメータ

| パラメータ | 型 | デフォルト | 説明 |
|----------|-------|----------|------|
| `enable` | boolean | `false` | デプロイメント戦略を有効化 |
| `strategy` | string | `"canary"` | 戦略（`ab_test`, `canary`） |
| `sticky_sessions` | boolean | `true` | スティッキーセッション（ユーザーが同じバリアントを受け取る） |

## 設定ファイル例

### 開発環境

```toml
[server]
host = "localhost"
port = 8080
workers = 2

[php]
libphp_path = "/usr/local/php-zts-embed/lib/libphp.dylib"
document_root = "./public"

[php.opcache]
validate_timestamps = true

[backend]
enable_hybrid = true
default_backend = "embedded"

[admin]
enable = true
unix_socket = "/tmp/fe-php-admin.sock"
http_port = 9001

[logging]
level = "debug"
format = "text"
```

### 本番環境

```toml
[server]
host = "0.0.0.0"
port = 443
workers = 16

[php]
libphp_path = "/usr/local/php-zts-embed/lib/libphp.so"
document_root = "/var/www/html"
worker_pool_size = 32

[php.opcache]
enable = true
memory_size = "512M"
validate_timestamps = false

[backend]
enable_hybrid = true
default_backend = "embedded"

[[backend.routing_rules]]
pattern = { type = "prefix", value = "/static/" }
backend = "static"
priority = 100

[[backend.routing_rules]]
pattern = { type = "prefix", value = "/api/" }
backend = "embedded"
priority = 90

[admin]
enable = true
unix_socket = "/var/run/fe-php-admin.sock"
http_port = 9001
allowed_ips = ["10.0.0.0/8"]

[metrics]
enable = true
port = 9090

[logging]
level = "info"
format = "json"
output = "/var/log/fe-php/access.log"

[waf]
enable = true
mode = "block"
rules_path = "/etc/fe-php/waf_rules.toml"

[tls]
enable = true
cert_path = "/etc/ssl/certs/server.crt"
key_path = "/etc/ssl/private/server.key"
alpn_protocols = ["h2", "http/1.1"]
```
