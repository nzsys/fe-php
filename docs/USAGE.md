# fe-php 使い方ガイド

本ガイドでは、fe-phpのインストールから運用までを詳しく説明します。

## 目次

1. [インストール](#インストール)
2. [基本的な使い方](#基本的な使い方)
3. [設定ファイル](#設定ファイル)
4. [デプロイ方法](#デプロイ方法)
5. [トラブルシューティング](#トラブルシューティング)

---

## インストール

### 必要要件

- **OS**: Linux (x86_64/ARM64), macOS (x86_64/Apple Silicon)
- **Rust**: 1.70以上
- **PHP**: 8.0以上（libphp使用時）
- **メモリ**: 最低1GB、推奨4GB以上
- **ディスク**: 100MB以上

### ソースからビルド

```bash
# 1. リポジトリをクローン
git clone https://github.com/nzsys/fe-php.git
cd fe-php

# 2. 依存関係を確認
rustc --version  # rustc 1.70以上が必要

# 3. リリースビルド
cargo build --release

# 4. バイナリのインストール
sudo cp target/release/fe-php /usr/local/bin/
sudo chmod +x /usr/local/bin/fe-php

# 5. 動作確認
fe-php --version
```

### バイナリ配布（将来対応予定）

```bash
# Linux x86_64
curl -L https://github.com/nzsys/fe-php/releases/download/v0.1.0/fe-php-linux-x86_64 -o fe-php
sudo mv fe-php /usr/local/bin/
sudo chmod +x /usr/local/bin/fe-php

# macOS (Apple Silicon)
curl -L https://github.com/nzsys/fe-php/releases/download/v0.1.0/fe-php-darwin-arm64 -o fe-php
sudo mv fe-php /usr/local/bin/
sudo chmod +x /usr/local/bin/fe-php
```

---

## 基本的な使い方

### サーバーの起動

#### デフォルト設定で起動

```bash
# デフォルトポート8080で起動
fe-php serve

# 出力例:
# [INFO] Starting fe-php server v0.1.0
# [INFO] Listening on http://0.0.0.0:8080
# [INFO] Worker pool initialized: 10 workers
# [INFO] Backend router initialized: embedded (default)
```

#### 設定ファイルを指定

```bash
fe-php serve --config /etc/fe-php/config.toml
```

#### コマンドラインオプション

```bash
# ヘルプ表示
fe-php serve --help

# ポート指定
fe-php serve --port 9000

# ホスト指定
fe-php serve --host 127.0.0.1 --port 8080

# ワーカー数指定
fe-php serve --workers 4

# ログレベル指定
fe-php serve --log-level debug
```

### 基本的な動作確認

```bash
# 1. ヘルスチェック
curl http://localhost:8080/health
# {"status":"healthy","version":"0.1.0","uptime_secs":123}

# 2. メトリクス確認
curl http://localhost:8080/metrics
# http_requests_total{method="GET",status="200"} 1

# 3. シンプルなPHPスクリプトを配置
echo '<?php echo "Hello, fe-php!"; ?>' > /var/www/html/hello.php
curl http://localhost:8080/hello.php
# Hello, fe-php!
```

### サーバーの停止

```bash
# 優雅なシャットダウン（既存リクエスト完了を待つ）
kill -TERM $(pgrep fe-php)

# または Ctrl+C
# fe-php serve を実行しているターミナルで Ctrl+C

# 強制停止（非推奨）
kill -9 $(pgrep fe-php)
```

---

## 設定ファイル

### 設定ファイルの場所

デフォルトの検索順序：

1. `./config.toml`（カレントディレクトリ）
2. `/etc/fe-php/config.toml`
3. `~/.config/fe-php/config.toml`

### 最小限の設定

```toml
# /etc/fe-php/config.toml

[server]
host = "0.0.0.0"
port = 8080
workers = 4

[php]
php_ini_path = "/etc/php/8.2/cli/php.ini"
pool_size = 10
```

### 完全な設定例

```toml
# サーバー基本設定
[server]
host = "0.0.0.0"
port = 8080
workers = 4  # 0 = CPU数と同じ

# PHP設定
[php]
php_ini_path = "/etc/php/8.2/cli/php.ini"
pool_size = 10
memory_limit = "128M"
max_execution_time = 30

# FastCGI設定（PHP-FPM使用時）
fastcgi_address = "unix:/var/run/php-fpm.sock"
# または TCP: "127.0.0.1:9000"

# 接続プール設定
[php.connection_pool]
max_size = 20
max_idle_time_secs = 60
max_lifetime_secs = 3600
connect_timeout_secs = 5

# ハイブリッドバックエンド設定
[backend]
enable_hybrid = true
default_backend = "embedded"

# 静的ファイル設定
[backend.static_files]
document_root = "/var/www/html"
index_files = ["index.html", "index.php"]
enable_etag = true
enable_range_request = true

# ルーティングルール（優先度順）
# APIエンドポイント → 高速なembedded
[[backend.routing_rules]]
pattern = { prefix = "/api/" }
backend = "embedded"
priority = 100

# 画像・CSS・JS → 静的ファイル配信
[[backend.routing_rules]]
pattern = { suffix = ".jpg" }
backend = "static"
priority = 90

[[backend.routing_rules]]
pattern = { suffix = ".css" }
backend = "static"
priority = 90

[[backend.routing_rules]]
pattern = { suffix = ".js" }
backend = "static"
priority = 90

# 管理画面 → 安定性重視でFastCGI
[[backend.routing_rules]]
pattern = { prefix = "/admin/" }
backend = "fastcgi"
priority = 80

# TLS/SSL設定
[tls]
cert_path = "/etc/letsencrypt/live/example.com/fullchain.pem"
key_path = "/etc/letsencrypt/live/example.com/privkey.pem"
http_redirect = true
http_port = 80

# ロギング設定
[logging]
level = "info"  # trace, debug, info, warn, error
format = "json"
output = "/var/log/fe-php/app.log"

# メトリクス設定
[metrics]
enabled = true
endpoint = "/metrics"

# WAF設定
[waf]
mode = "block"  # off, learn, detect, block
log_blocked = true

# CORS設定
[cors]
enabled = true
allowed_origins = ["https://example.com"]
allowed_methods = ["GET", "POST", "PUT", "DELETE", "OPTIONS"]
allowed_headers = ["Content-Type", "Authorization"]
max_age = 3600
allow_credentials = true

# レート制限
[rate_limit]
enabled = true
requests_per_second = 100
burst_size = 200

# IPフィルタリング
[security]
ip_filter_mode = "whitelist"  # whitelist, blacklist, off
allowed_ips = [
    "192.168.0.0/16",
    "10.0.0.0/8",
]

# 圧縮設定
[compression]
enable_gzip = true
enable_brotli = true
min_size = 1024
gzip_level = 6
brotli_quality = 6
```

### 設定の検証

```bash
# 設定ファイルの文法チェック
fe-php validate --config /etc/fe-php/config.toml

# 設定を表示（パスワードなどはマスク）
fe-php show-config --config /etc/fe-php/config.toml
```

### 設定のリロード

```bash
# 実行中のサーバーに設定リロードシグナルを送信
kill -USR1 $(pgrep fe-php)

# systemdの場合
systemctl reload fe-php

# ログで確認
tail -f /var/log/fe-php/app.log
# [INFO] Received SIGUSR1 signal, triggering config reload
# [INFO] Configuration reloaded successfully
```

---

## Admin Console

### 概要

fe-phpには、Webベースの管理コンソールが組み込まれています。サーバーの状態、メトリクス、バックエンド情報をリアルタイムで監視できます。

### アクセス方法

```bash
# Admin Console有効化
fe-php serve --config /etc/fe-php/config.toml

# ブラウザでアクセス
# http://localhost:9002/
```

### 設定

```toml
[admin]
enable = true
host = "127.0.0.1"      # セキュリティのためlocalhostのみ
http_port = 9002        # 管理コンソールポート
allowed_ips = ["127.0.0.1"]  # アクセス許可IP
```

**セキュリティ上の注意**:
- デフォルトで `127.0.0.1` (localhost) にバインドされます
- 外部ネットワークからアクセスする場合は、必ずリバースプロキシ（Nginx等）を経由させてください
- 本番環境では `host = "127.0.0.1"` を維持することを推奨します

### 利用可能な機能

#### 1. Dashboard (/)

HTMLベースの管理画面：
- **サーバー情報**: バージョン、稼働時間、PID、起動日時
- **リアルタイムメトリクス**: RPS、アクティブ接続数、総リクエスト数、エラー率
- **バックエンド状態**: 各バックエンドの健全性、リクエスト数、エラー数、平均応答時間

```bash
# ブラウザでアクセス
open http://localhost:9002/
```

#### 2. JSON API (/api/status)

プログラマティックアクセス用のJSON API：

```bash
# ステータス取得
curl http://localhost:9002/api/status | jq

# 出力例
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
      "name": "embedded",
      "backend_type": "embedded",
      "status": "healthy",
      "requests": 320000,
      "errors": 42,
      "avg_response_ms": 1.2
    }
  ]
}
```

### 監視システムとの連携

Admin Console APIを監視システムと連携：

```bash
# Prometheus Exporterとして利用
# (将来実装予定: /api/metrics エンドポイント)

# カスタムスクリプト例
#!/bin/bash
STATUS=$(curl -s http://localhost:9002/api/status)
ERROR_RATE=$(echo $STATUS | jq -r '.metrics.error_rate')

if (( $(echo "$ERROR_RATE > 1.0" | bc -l) )); then
  echo "High error rate detected: $ERROR_RATE%"
  # アラート送信処理
fi
```

### 読み取り専用について

Admin Consoleは読み取り専用インターフェースです。

- 設定変更不可
- サーバー再起動不可
- ログレベル変更不可
- ステータス表示のみ

設定変更は、設定ファイルを編集して `kill -USR1 <pid>` でリロードしてください。

### 計画中の機能

将来のバージョンで以下の機能が追加予定：
- `/logs` - リアルタイムログビューア
- `/metrics` - グラフィカルメトリクス表示
- `/waf` - WAFルール管理
- `/backends` - バックエンド詳細管理
- `/system` - システムリソース情報

---

## デプロイ方法

### systemdサービスとして実行

#### サービスファイル作成

```bash
sudo nano /etc/systemd/system/fe-php.service
```

```ini
[Unit]
Description=fe-php Application Server
Documentation=https://github.com/nzsys/fe-php
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=www-data
Group=www-data
WorkingDirectory=/var/www/html

# 環境変数
Environment="RUST_LOG=info"
Environment="RUST_BACKTRACE=1"

# 実行コマンド
ExecStart=/usr/local/bin/fe-php serve --config /etc/fe-php/config.toml

# リロード
ExecReload=/bin/kill -USR1 $MAINPID

# 再起動設定
Restart=always
RestartSec=5
StartLimitInterval=0

# セキュリティ強化
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/log/fe-php /var/www/html

# リソース制限
LimitNOFILE=65536
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
```

#### サービスの管理

```bash
# サービスを有効化
sudo systemctl enable fe-php

# サービス起動
sudo systemctl start fe-php

# ステータス確認
sudo systemctl status fe-php

# ログ確認
sudo journalctl -u fe-php -f

# 設定リロード
sudo systemctl reload fe-php

# 再起動
sudo systemctl restart fe-php

# サービス停止
sudo systemctl stop fe-php
```

### Dockerコンテナとして実行

#### Dockerfile

```dockerfile
FROM rust:1.75-slim as builder

WORKDIR /app
COPY . .

# 依存関係インストール
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# ビルド
RUN cargo build --release

# ランタイムイメージ
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    php8.2-cli \
    php8.2-fpm \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/fe-php /usr/local/bin/
COPY config.toml /etc/fe-php/

EXPOSE 8080
EXPOSE 8443

CMD ["fe-php", "serve", "--config", "/etc/fe-php/config.toml"]
```

#### docker-compose.yml

```yaml
version: '3.8'

services:
  fe-php:
    build: .
    ports:
      - "8080:8080"
      - "8443:8443"
    volumes:
      - ./config.toml:/etc/fe-php/config.toml:ro
      - ./html:/var/www/html:ro
      - ./logs:/var/log/fe-php
    environment:
      - RUST_LOG=info
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
```

```bash
# コンテナ起動
docker-compose up -d

# ログ確認
docker-compose logs -f

# コンテナ停止
docker-compose down
```

### Nginxリバースプロキシ設定

```nginx
upstream fe-php {
    server 127.0.0.1:8080;
    keepalive 32;
}

server {
    listen 80;
    server_name example.com;

    # Let's Encrypt認証用
    location /.well-known/acme-challenge/ {
        root /var/www/html;
    }

    # その他は全てHTTPSへリダイレクト
    location / {
        return 301 https://$host$request_uri;
    }
}

server {
    listen 443 ssl http2;
    server_name example.com;

    # SSL証明書
    ssl_certificate /etc/letsencrypt/live/example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/example.com/privkey.pem;

    # セキュリティヘッダー
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;

    # fe-phpへプロキシ
    location / {
        proxy_pass http://fe-php;
        proxy_http_version 1.1;
        proxy_set_header Connection "";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # タイムアウト
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }

    # 静的ファイルはNginxで直接配信（オプション）
    location ~* \.(jpg|jpeg|png|gif|ico|css|js|svg|woff|woff2|ttf)$ {
        root /var/www/html;
        expires 1y;
        add_header Cache-Control "public, immutable";
    }
}
```

---

## トラブルシューティング

### よくある問題と解決方法

#### 1. サーバーが起動しない

**エラー**: `Error: Permission denied (os error 13)`

**原因**: ポート80/443はroot権限が必要

**解決方法**:
```bash
# 方法1: sudoで起動
sudo fe-php serve --config /etc/fe-php/config.toml

# 方法2: setcapでポートバインド許可
sudo setcap 'cap_net_bind_service=+ep' /usr/local/bin/fe-php

# 方法3: 1024以上のポートを使用
fe-php serve --port 8080
```

#### 2. PHPスクリプトが実行されない

**症状**: PHPファイルがダウンロードされる、または404エラー

**確認事項**:
```bash
# 1. PHP拡張子のルーティング確認
curl -I http://localhost:8080/test.php

# 2. ドキュメントルート確認
cat /etc/fe-php/config.toml | grep document_root

# 3. ファイル権限確認
ls -la /var/www/html/test.php
# -rw-r--r-- www-data www-data が望ましい
```

**解決方法**:
```toml
# config.toml
[backend]
default_backend = "embedded"  # または "fastcgi"

[backend.static_files]
document_root = "/var/www/html"
```

#### 3. FastCGI接続エラー

**エラー**: `Failed to connect to FastCGI at 127.0.0.1:9000`

**確認事項**:
```bash
# PHP-FPMが起動しているか確認
systemctl status php8.2-fpm

# ソケット確認
ls -la /var/run/php-fpm.sock

# 接続テスト
telnet 127.0.0.1 9000
```

**解決方法**:
```bash
# PHP-FPM起動
sudo systemctl start php8.2-fpm

# Unixソケット使用に変更
# config.toml
[php]
fastcgi_address = "unix:/var/run/php-fpm.sock"
```

#### 4. メモリ不足

**症状**: プロセスがクラッシュ、OOM Killer発動

**確認方法**:
```bash
# メモリ使用量確認
ps aux | grep fe-php

# システムログ確認
dmesg | grep -i "out of memory"
```

**解決方法**:
```toml
# config.toml - ワーカー数を減らす
[server]
workers = 2  # デフォルト4から削減

[php]
pool_size = 5  # デフォルト10から削減
memory_limit = "64M"  # PHPメモリ制限
```

#### 5. 高負荷時のパフォーマンス低下

**調査方法**:
```bash
# メトリクス確認
curl http://localhost:8080/metrics | grep backend_request_duration

# プロファイリング
perf record -g -p $(pgrep fe-php)
perf report
```

**最適化**:
```toml
[server]
workers = 8  # CPU数の2倍

[php.connection_pool]
max_size = 50  # プールサイズ増加

[compression]
enable_gzip = false  # CPU節約のため圧縮無効化

[backend]
default_backend = "fastcgi"  # 安定性優先
```

### デバッグモード

```bash
# デバッグログ有効化
RUST_LOG=debug fe-php serve --config /etc/fe-php/config.toml

# トレースログ（非常に詳細）
RUST_LOG=trace fe-php serve --config /etc/fe-php/config.toml

# 特定モジュールのみ
RUST_LOG=fe_php::backend=debug fe-php serve
```

### ログの確認

```bash
# systemd journal
sudo journalctl -u fe-php -f

# ファイルログ
tail -f /var/log/fe-php/app.log

# エラーのみ抽出
grep ERROR /var/log/fe-php/app.log
```

---

- [ARCHITECTURE.md](./ARCHITECTURE.md) アーキテクチャ
- [FEATURES.md](./FEATURES.md) 全機能
