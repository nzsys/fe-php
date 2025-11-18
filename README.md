# fe-php

ハイブリッドPHPアプリケーションサーバー

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![PHP](https://img.shields.io/badge/php-8.0%2B-777BB4.svg)](https://www.php.net/)

## 概要

fe-phpは、Rustで実装された高性能PHPアプリケーションサーバーです。従来のNginx + PHP-FPMの組み合わせを単一バイナリに統合し、3つの異なるバックエンド（Embedded PHP、FastCGI、Static Files）をリクエストパスに応じて自動選択するハイブリッドアーキテクチャを採用しています。

```
従来のスタック:                    fe-php:
┌──────────────┐                  ┌──────────────────────┐
│   Nginx      │                  │                      │
└──────┬───────┘                  │     fe-php           │
       │                          │   (Single Binary)    │
┌──────┴───────┐                  │                      │
│  PHP-FPM     │                  │  • HTTP Server       │
└──────────────┘                  │  • PHP Runtime       │
                                  │  • Static Files      │
複数プロセス                       │  • WAF               │
複雑な設定                         │  • Metrics           │
                                  │  • Admin API         │
                                  │  • TUI Monitor       │
                                  └──────────────────────┘
                                  単一バイナリ
                                  統合設定
```

## 主要機能

### ハイブリッドバックエンド

リクエストパスに応じて最適なバックエンドを自動選択：

| バックエンド | 特徴 | 用途 |
|------------|------|------|
| Embedded | libphpを直接実行。最も高速だがメモリを共有 | API、軽量な処理 |
| FastCGI | PHP-FPMへプロキシ。プロセス分離で安定性重視 | 管理画面、長時間処理 |
| Static | Rustで直接ファイル配信。PHPオーバーヘッドなし | 画像、CSS、JavaScript |

設定例：

```toml
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/api/" }
backend = "embedded"
priority = 100

[[backend.routing_rules]]
pattern = { type = "suffix", value = ".jpg" }
backend = "static"
priority = 90

[[backend.routing_rules]]
pattern = { type = "prefix", value = "/admin/" }
backend = "fastcgi"
priority = 80
```

### パフォーマンス

テスト環境: Apple M1 Max、32GB RAM、macOS Sequoia 15.1

高並列負荷（500 RPS目標、並列50）:

| サーバー | 実測RPS | Nginx比 | p50レイテンシ | p99レイテンシ |
|---------|--------|---------|-------------|-------------|
| Nginx + PHP-FPM | 209.86 | 1.0x | 4ms | 10ms |
| fe-php (Hybrid) | 374.80 | 1.79x | 2ms | 3ms |

ストレステスト（1000 RPS目標、並列100）:

| サーバー | 実測RPS | Nginx比 | p50レイテンシ | p99レイテンシ |
|---------|--------|---------|-------------|-------------|
| Nginx + PHP-FPM | 201.18 | 1.0x | 4ms | 11ms |
| fe-php (Embedded) | 429.76 | 2.14x | 1ms | 5ms |

### 監視機能

**TUI Monitor**: ターミナルベースのリアルタイム監視ツール

```bash
# ローカル監視
fe-php monitor

# Unix Socket経由でリモート監視
fe-php monitor --socket /var/run/fe-php-admin.sock

# SSH経由でのリモート監視
ssh production-server "fe-php monitor --socket /var/run/fe-php-admin.sock"

# JSON形式で出力（スクリプト連携用）
fe-php monitor --format json --socket /var/run/fe-php-admin.sock
```

主な機能:
- サーバー稼働時間、リクエスト数、エラー率のリアルタイム表示
- バックエンド別のメトリクス（リクエスト数、エラー数、平均応答時間）
- リクエストログビューア（ステータスコード別色分け、最新100件）
- ログ自動分析（エンドポイント別統計、遅延リクエスト検出、不審なアクティビティ検出）
- ワーカー状態表示
- ブロック済みIP一覧

**Admin API**: Unix SocketまたはHTTP経由での管理インターフェース

```bash
# Unix Socket経由（推奨）
echo '{"command":"reload_config"}' | socat - UNIX-CONNECT:/var/run/fe-php-admin.sock
echo '{"command":"status"}' | socat - UNIX-CONNECT:/var/run/fe-php-admin.sock

# HTTP API経由（外部ツール連携用）
curl http://localhost:9001/api/status
curl http://localhost:9001/api/health
```

提供されるAPI:
- `/api/status`: サーバー状態、メトリクス、バックエンド情報
- `/api/health`: ヘルスチェック
- `/api/logs/recent`: 最近のリクエストログ
- `/api/logs/analysis`: ログ分析結果
- `/api/security/blocked-ips`: ブロック済みIP一覧
- `/metrics`: Prometheus形式のメトリクス

**Prometheusメトリクス**: 詳細なパフォーマンス測定

```bash
curl http://localhost:9090/_metrics
```

提供されるメトリクス:
- `active_connections`: アクティブ接続数
- `backend_requests_total`: バックエンド別リクエスト総数
- `backend_errors_total`: バックエンド別エラー総数
- `backend_request_duration_seconds`: バックエンド別リクエスト処理時間（ヒストグラム）
- `http_requests_total`: HTTPリクエスト総数
- `process_*`: プロセスメトリクス（CPU、メモリ）

### セキュリティ機能

**WAF (Web Application Firewall)**: リクエストの検査とブロック

- SQLインジェクション検出
- XSS (Cross-Site Scripting) 検出
- パストラバーサル検出
- カスタムルール定義

**レート制限**: IP別リクエスト制限

```toml
[waf.rate_limit]
requests_per_ip = 100
window_seconds = 60
burst = 20
```

**IP制限**: CIDR表記によるホワイトリスト/ブロックリスト

```toml
[admin]
allowed_ips = ["127.0.0.1", "::1", "192.168.1.0/24"]
```

**動的IPブロック**: Admin API経由でのランタイムブロック

```bash
echo '{"command":"block_ip","ip":"192.168.1.100"}' | socat - UNIX-CONNECT:/var/run/fe-php-admin.sock
echo '{"command":"unblock_ip","ip":"192.168.1.100"}' | socat - UNIX-CONNECT:/var/run/fe-php-admin.sock
```

### その他の機能

- TLS/SSL対応（Let's Encrypt対応、SNI対応）
- CORS対応（オリジン/メソッド/ヘッダーの詳細制御）
- グレースフルシャットダウン（SIGTERM/SIGINT対応）
- 設定ホットリロード（Admin API経由）
- ワーカープロセス再起動（Admin API経由）
- ロードバランシング（Round Robin、Least Connections、Weighted Round Robin、IP Hash）
- サーキットブレーカー
- A/Bテスト・カナリーリリース対応
- OpenTelemetry分散トレーシング
- Redis統合（セッション管理）
- GeoIPフィルタリング

## クイックスタート

### 必要要件

- Rust 1.75以上
- PHP 8.0以上（ZTS版、embed SAPI有効化）
- Linux、macOSまたはその他のUnix系OS

### インストール

```bash
git clone https://github.com/nzsys/fe-php.git
cd fe-php
cargo build --release
sudo cp target/release/fe-php /usr/local/bin/
```

### 最小設定

`config.toml`:

```toml
[server]
host = "0.0.0.0"
port = 8080
workers = 4

[php]
libphp_path = "/usr/local/php-zts-embed/lib/libphp.so"  # Linux
# libphp_path = "/usr/local/php-zts-embed/lib/libphp.dylib"  # macOS
document_root = "/var/www/html"

[backend]
enable_hybrid = true
default_backend = "embedded"

[admin]
enable = true
unix_socket = "/var/run/fe-php-admin.sock"
http_port = 9001
```

### サーバー起動

```bash
fe-php serve --config config.toml
```

### 動作確認

```bash
# ヘルスチェック
curl http://localhost:8080/_health

# TUI Monitor起動
fe-php monitor --socket /var/run/fe-php-admin.sock

# メトリクス確認
curl http://localhost:9090/_metrics

# Admin API確認
curl http://localhost:9001/api/status | jq .
```

## 設定例

### ハイブリッドモード設定

```toml
[backend]
enable_hybrid = true
default_backend = "embedded"

# 静的ファイルは直接配信
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/static/" }
backend = "static"
priority = 100

[[backend.routing_rules]]
pattern = { type = "suffix", value = ".css" }
backend = "static"
priority = 90

[[backend.routing_rules]]
pattern = { type = "suffix", value = ".js" }
backend = "static"
priority = 90

[[backend.routing_rules]]
pattern = { type = "suffix", value = ".jpg" }
backend = "static"
priority = 90

# APIエンドポイントは高速なEmbeddedバックエンド
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/api/" }
backend = "embedded"
priority = 80

# 管理画面はプロセス分離されたFastCGI
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/admin/" }
backend = "fastcgi"
priority = 70

[backend.static_files]
enable = true
root = "/var/www/html"
index_files = ["index.html", "index.htm"]
```

### WAF設定

```toml
[waf]
enable = true
mode = "block"  # または "detect"
rules_path = "waf_rules.toml"

[waf.rate_limit]
requests_per_ip = 100
window_seconds = 60
burst = 20
```

`waf_rules.toml`:

```toml
[[rules]]
id = "SQL_INJECTION"
pattern = "(?i)(union|select|insert|update|delete|drop|create|alter)\\s+"
severity = "high"
action = "block"

[[rules]]
id = "XSS"
pattern = "(?i)<script|javascript:|onerror=|onload="
severity = "high"
action = "block"
```

### TLS/SSL設定

```toml
[tls]
enable = true
cert_path = "/etc/ssl/certs/server.crt"
key_path = "/etc/ssl/private/server.key"
alpn_protocols = ["h2", "http/1.1"]
http_redirect = true
http_port = 80
```

## コマンドラインインターフェース

### サーバー起動

```bash
fe-php serve [OPTIONS]

OPTIONS:
  -c, --config <FILE>    設定ファイルのパス [default: config.toml]
  -h, --help             ヘルプメッセージを表示
```

### Monitor

```bash
fe-php monitor [OPTIONS]

OPTIONS:
  -s, --socket <PATH>    Unix Socketパス（リモート監視用）
  -f, --format <FORMAT>  出力形式 [default: tui] [possible: tui, json, text]
  -h, --help             ヘルプメッセージを表示
```

### ベンチマーク

```bash
fe-php bench [OPTIONS]

OPTIONS:
  -u, --url <URL>              ベンチマーク対象のURL
  -d, --duration <SECONDS>     実行時間（秒）[default: 30]
  -r, --rps <RPS>              目標RPS [default: 100]
  -c, --concurrency <NUM>      並列数 [default: 10]
  -h, --help                   ヘルプメッセージを表示
```

## ユースケース

### SaaS/Webアプリケーション

API、静的ファイル、管理画面を1つのバイナリで提供。APIは高速なEmbeddedバックエンド、静的ファイルは直接配信、管理画面はプロセス分離されたFastCGIで安定性を確保。

### WordPress

公開ページは高速なEmbeddedバックエンド、管理画面はFastCGI、アップロードファイルは直接配信。

### ECサイト

商品APIは超高速なEmbeddedバックエンド、商品画像は直接配信、決済処理はFastCGIで安定性重視。

## アーキテクチャ

```
┌──────────────────────────────────────────────────────────────────────┐
│                          fe-php Server                                │
├──────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  ┌───────────────────┐         ┌──────────────────────────────┐      │
│  │  Request Router   │────────▶│   Pattern Matcher            │      │
│  └───────────────────┘         │   (prefix/suffix/regex)      │      │
│          │                     └──────────────────────────────┘      │
│          │                                                            │
│          ├──────────┬───────────────┬───────────────────┐           │
│          │          │               │                   │           │
│          ▼          ▼               ▼                   ▼           │
│  ┌──────────┐ ┌──────────┐  ┌──────────┐     ┌────────────────┐   │
│  │ Embedded │ │ FastCGI  │  │  Static  │     │   Admin API    │   │
│  │ Backend  │ │ Backend  │  │ Backend  │     │ (Unix Socket + │   │
│  └──────────┘ └──────────┘  └──────────┘     │  HTTP JSON)    │   │
│       │             │              │          └────────────────┘   │
│       ▼             ▼              ▼                   │            │
│  ┌──────────┐ ┌──────────┐  ┌──────────┐             ▼            │
│  │  libphp  │ │ PHP-FPM  │  │   Disk   │      ┌────────────────┐  │
│  │ (in-proc)│ │ (socket) │  │  (read)  │      │  TUI Monitor   │  │
│  └──────────┘ └──────────┘  └──────────┘      │  External Tools│  │
│                                                └────────────────┘  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## ドキュメント

- [Getting Started](docs/getting-started.md) - インストールとセットアップ
- [Configuration](docs/configuration.md) - 設定ガイド
- [Backends](docs/backends.md) - バックエンド設定
- [Monitoring](docs/monitoring.md) - 監視とメトリクス
- [Security](docs/security.md) - セキュリティ機能
- [Deployment](docs/deployment.md) - デプロイメント
- [API Reference](docs/api-reference.md) - Admin APIリファレンス

## ライセンス

MIT License - 詳細は[LICENSE](LICENSE)ファイルを参照してください。
