# Deployment

fe-phpを本番環境にデプロイする方法を説明します。

## システム要件

### ハードウェア

最小要件:
- CPU: 2コア以上
- メモリ: 2GB以上
- ストレージ: 10GB以上

推奨要件:
- CPU: 4コア以上
- メモリ: 8GB以上
- ストレージ: 50GB以上（SSD推奨）

### ソフトウェア

- Linux（Ubuntu 20.04+ または CentOS 8+）
- PHP 8.0以上（ZTS版、embed SAPI有効化）
- Systemd（サービス管理用）

## インストール

### 1. PHP ZTSのインストール

#### Ubuntu/Debian

```bash
# ビルド依存関係
sudo apt-get update
sudo apt-get install -y build-essential autoconf libtool bison re2c \
    libxml2-dev libsqlite3-dev libssl-dev libcurl4-openssl-dev \
    libpng-dev libjpeg-dev libonig-dev libzip-dev

# PHPソースコード取得
cd /tmp
wget https://www.php.net/distributions/php-8.3.0.tar.gz
tar -xzf php-8.3.0.tar.gz
cd php-8.3.0

# ZTS有効化、embed SAPI有効化でビルド
./configure \
    --prefix=/usr/local/php-zts-embed \
    --enable-embed=shared \
    --enable-zts \
    --with-openssl \
    --with-curl \
    --with-zlib \
    --enable-mbstring \
    --with-mysqli \
    --enable-opcache

make -j$(nproc)
sudo make install

# 共有ライブラリパスの設定
echo "/usr/local/php-zts-embed/lib" | sudo tee /etc/ld.so.conf.d/php-zts.conf
sudo ldconfig
```

#### CentOS/RHEL

```bash
# ビルド依存関係
sudo yum groupinstall -y "Development Tools"
sudo yum install -y openssl-devel libxml2-devel sqlite-devel \
    curl-devel libpng-devel libjpeg-devel oniguruma-devel libzip-devel

# 以下はUbuntu/Debianと同様
```

### 2. fe-phpのビルド

```bash
# Rustのインストール
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# fe-phpのクローン
git clone https://github.com/nzsys/fe-php.git
cd fe-php

# リリースビルド
cargo build --release

# バイナリのインストール
sudo cp target/release/fe-php /usr/local/bin/
sudo chmod +x /usr/local/bin/fe-php
```

### 3. ユーザーとディレクトリの作成

```bash
# 専用ユーザーの作成
sudo useradd -r -s /bin/false fe-php

# ディレクトリ作成
sudo mkdir -p /var/www/html
sudo mkdir -p /etc/fe-php
sudo mkdir -p /var/log/fe-php
sudo mkdir -p /var/run/fe-php

# 権限設定
sudo chown -R fe-php:fe-php /var/www/html
sudo chown -R fe-php:fe-php /var/log/fe-php
sudo chown -R fe-php:fe-php /var/run/fe-php
```

## 設定ファイル

### 本番環境用設定

`/etc/fe-php/config.toml`:

```toml
[server]
host = "0.0.0.0"
port = 443
workers = 16  # CPU数 × 2

[php]
libphp_path = "/usr/local/php-zts-embed/lib/libphp.so"
document_root = "/var/www/html"
worker_pool_size = 32
worker_max_requests = 10000

[php.opcache]
enable = true
memory_size = "512M"
max_files = 20000
validate_timestamps = false

[backend]
enable_hybrid = true
default_backend = "embedded"

# 静的ファイル
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

[[backend.routing_rules]]
pattern = { type = "suffix", value = ".png" }
backend = "static"
priority = 90

# APIエンドポイント
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/api/" }
backend = "embedded"
priority = 80

# 管理画面
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/admin/" }
backend = "fastcgi"
priority = 70

[backend.static_files]
enable = true
root = "/var/www/html"
index_files = ["index.html", "index.htm"]

[backend.connection_pool]
max_size = 100
max_idle_time_secs = 600
max_lifetime_secs = 3600
connect_timeout_secs = 10

[admin]
enable = true
unix_socket = "/var/run/fe-php/admin.sock"
# HTTP APIは無効化（セキュリティ）

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

[waf.rate_limit]
requests_per_ip = 100
window_seconds = 60
burst = 20

[tls]
enable = true
cert_path = "/etc/letsencrypt/live/example.com/fullchain.pem"
key_path = "/etc/letsencrypt/live/example.com/privkey.pem"
alpn_protocols = ["h2", "http/1.1"]
http_redirect = true
http_port = 80

[geoip]
enable = false
database_path = "/usr/share/GeoIP/GeoLite2-Country.mmdb"
allowed_countries = []
blocked_countries = []
```

### WAFルールファイル

`/etc/fe-php/waf_rules.toml`:

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

[[rules]]
id = "PATH_TRAVERSAL"
pattern = "\\.\\./|\\.\\.\\\\"
severity = "high"
action = "block"
```

## Systemdサービス

### サービスファイルの作成

`/etc/systemd/system/fe-php.service`:

```ini
[Unit]
Description=fe-php Application Server
After=network.target

[Service]
Type=simple
User=fe-php
Group=fe-php
WorkingDirectory=/var/www/html
ExecStart=/usr/local/bin/fe-php serve --config /etc/fe-php/config.toml
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure
RestartSec=5s
StandardOutput=journal
StandardError=journal

# セキュリティ設定
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/www/html /var/log/fe-php /var/run/fe-php

# リソース制限
LimitNOFILE=65536
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
```

### サービスの有効化と起動

```bash
# サービスファイルのリロード
sudo systemctl daemon-reload

# サービスの有効化（起動時に自動起動）
sudo systemctl enable fe-php

# サービスの起動
sudo systemctl start fe-php

# ステータス確認
sudo systemctl status fe-php

# ログ確認
sudo journalctl -u fe-php -f
```

### サービスの管理

```bash
# 起動
sudo systemctl start fe-php

# 停止
sudo systemctl stop fe-php

# 再起動
sudo systemctl restart fe-php

# 設定リロード（Admin API経由）
echo '{"command":"reload_config"}' | sudo socat - UNIX-CONNECT:/var/run/fe-php/admin.sock

# ログ確認
sudo journalctl -u fe-php -n 100 --no-pager
```

## TLS/SSL証明書

### Let's Encryptの使用

#### Certbotのインストール

```bash
# Ubuntu/Debian
sudo apt-get install certbot

# CentOS/RHEL
sudo yum install certbot
```

#### 証明書の取得

```bash
# fe-phpを一時停止
sudo systemctl stop fe-php

# スタンドアロンモードで証明書取得
sudo certbot certonly --standalone -d example.com -d www.example.com

# 証明書のパス
# /etc/letsencrypt/live/example.com/fullchain.pem
# /etc/letsencrypt/live/example.com/privkey.pem

# fe-phpを再起動
sudo systemctl start fe-php
```

#### 自動更新の設定

```bash
# cron設定
sudo crontab -e

# 以下を追加（毎日午前2時に実行）
0 2 * * * certbot renew --quiet --pre-hook "systemctl stop fe-php" --post-hook "systemctl start fe-php"
```

### カスタム証明書の使用

```bash
# 証明書ファイルのコピー
sudo cp server.crt /etc/ssl/certs/fe-php.crt
sudo cp server.key /etc/ssl/private/fe-php.key

# 権限設定
sudo chmod 644 /etc/ssl/certs/fe-php.crt
sudo chmod 600 /etc/ssl/private/fe-php.key
sudo chown root:root /etc/ssl/certs/fe-php.crt
sudo chown root:fe-php /etc/ssl/private/fe-php.key

# 設定ファイルの更新
[tls]
cert_path = "/etc/ssl/certs/fe-php.crt"
key_path = "/etc/ssl/private/fe-php.key"
```

## ログローテーション

### logrotateの設定

`/etc/logrotate.d/fe-php`:

```
/var/log/fe-php/*.log {
    daily
    missingok
    rotate 30
    compress
    delaycompress
    notifempty
    create 0640 fe-php fe-php
    sharedscripts
    postrotate
        /bin/systemctl reload fe-php > /dev/null 2>&1 || true
    endscript
}
```

### 手動でのローテーション実行

```bash
sudo logrotate -f /etc/logrotate.d/fe-php
```

## 監視

### Prometheusの設定

`/etc/prometheus/prometheus.yml`:

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'fe-php'
    static_configs:
      - targets: ['localhost:9090']
    metrics_path: '/_metrics'
```

### Grafanaダッシュボード

主要なメトリクスの可視化：

1. リクエストレート
2. エラー率
3. レイテンシ（P50、P95、P99）
4. アクティブ接続数
5. バックエンド別メトリクス

### アラート設定

`/etc/prometheus/alert.rules`:

```yaml
groups:
  - name: fe-php
    interval: 30s
    rules:
      - alert: HighErrorRate
        expr: rate(backend_errors_total[5m]) / rate(backend_requests_total[5m]) > 0.05
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High error rate"

      - alert: HighLatency
        expr: histogram_quantile(0.99, rate(backend_request_duration_seconds_bucket[5m])) > 0.5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High latency"
```

## バックアップ

### 設定ファイルのバックアップ

```bash
#!/bin/bash
# backup.sh

BACKUP_DIR="/backup/fe-php"
DATE=$(date +%Y%m%d_%H%M%S)

mkdir -p $BACKUP_DIR

# 設定ファイル
tar -czf $BACKUP_DIR/config_$DATE.tar.gz \
  /etc/fe-php/ \
  /etc/systemd/system/fe-php.service

# TLS証明書
tar -czf $BACKUP_DIR/tls_$DATE.tar.gz \
  /etc/letsencrypt/live/

# ログ（直近7日分）
find /var/log/fe-php/ -name "*.log*" -mtime -7 | \
  tar -czf $BACKUP_DIR/logs_$DATE.tar.gz -T -

# 古いバックアップの削除（30日以上前）
find $BACKUP_DIR -name "*.tar.gz" -mtime +30 -delete
```

cron設定:
```bash
# 毎日午前3時に実行
0 3 * * * /usr/local/bin/backup.sh
```

### アプリケーションデータのバックアップ

```bash
# ドキュメントルート
tar -czf /backup/fe-php/www_$DATE.tar.gz /var/www/html

# データベース（MySQLの例）
mysqldump -u root -p database_name | gzip > /backup/fe-php/db_$DATE.sql.gz
```

## スケーリング

### 垂直スケーリング（スケールアップ）

より強力なサーバーへ移行：

1. ワーカー数を増やす:
```toml
[server]
workers = 32  # CPU数に応じて調整

[php]
worker_pool_size = 64
```

2. メモリを増やす:
```toml
[php.opcache]
memory_size = "1G"
```

### 水平スケーリング（スケールアウト）

複数のサーバーでロードバランシング：

```
┌─────────────┐
│  Load       │
│  Balancer   │
│  (Nginx)    │
└──────┬──────┘
       │
       ├───────────┬───────────┬───────────┐
       │           │           │           │
       ▼           ▼           ▼           ▼
  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐
  │ fe-php  │ │ fe-php  │ │ fe-php  │ │ fe-php  │
  │ Server1 │ │ Server2 │ │ Server3 │ │ Server4 │
  └─────────┘ └─────────┘ └─────────┘ └─────────┘
```

Nginx設定例:

```nginx
upstream fe_php_backend {
    least_conn;
    server 192.168.1.10:443 max_fails=3 fail_timeout=30s;
    server 192.168.1.11:443 max_fails=3 fail_timeout=30s;
    server 192.168.1.12:443 max_fails=3 fail_timeout=30s;
    server 192.168.1.13:443 max_fails=3 fail_timeout=30s;
}

server {
    listen 443 ssl http2;
    server_name example.com;

    ssl_certificate /etc/ssl/certs/example.com.crt;
    ssl_certificate_key /etc/ssl/private/example.com.key;

    location / {
        proxy_pass https://fe_php_backend;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

## トラブルシューティング

### サービスが起動しない

```bash
# ログ確認
sudo journalctl -u fe-php -n 100 --no-pager

# 設定ファイルの検証
/usr/local/bin/fe-php serve --config /etc/fe-php/config.toml --dry-run

# 権限確認
sudo -u fe-php /usr/local/bin/fe-php serve --config /etc/fe-php/config.toml
```

### メモリ不足

```bash
# メモリ使用量確認
sudo systemctl status fe-php
ps aux | grep fe-php

# OOM Killerログ確認
sudo dmesg | grep -i "out of memory"

# 対策: ワーカー数を減らす
[server]
workers = 8  # 減らす

[php]
worker_pool_size = 16  # 減らす
```

### 高いCPU使用率

```bash
# CPU使用率確認
top -p $(pgrep fe-php)

# プロファイリング（開発環境で実行）
perf record -g -p $(pgrep fe-php)
perf report

# 対策: ワーカー数を最適化
[server]
workers = 16  # CPU数 × 2 を目安
```

### ディスク容量不足

```bash
# ディスク使用量確認
df -h
du -sh /var/log/fe-php/*

# ログの削除
sudo find /var/log/fe-php/ -name "*.log.*" -mtime +7 -delete

# ログローテーション設定の見直し
[logging]
output = "/var/log/fe-php/access.log"
```

## パフォーマンスチューニング

### OS レベル

#### ファイルディスクリプタ制限

`/etc/security/limits.conf`:

```
fe-php soft nofile 65536
fe-php hard nofile 65536
```

#### ネットワークチューニング

`/etc/sysctl.conf`:

```
# TCP設定
net.core.somaxconn = 65535
net.ipv4.tcp_max_syn_backlog = 65535
net.ipv4.ip_local_port_range = 1024 65535
net.ipv4.tcp_tw_reuse = 1
net.ipv4.tcp_fin_timeout = 30

# メモリ設定
vm.swappiness = 10
vm.vfs_cache_pressure = 50
```

適用:
```bash
sudo sysctl -p
```

### アプリケーションレベル

#### ワーカー数の最適化

```toml
[server]
workers = 16  # CPU数 × 1.5 〜 2.0

[php]
worker_pool_size = 32  # server.workers × 2
```

#### OPcacheの最適化

```toml
[php.opcache]
enable = true
memory_size = "512M"  # アプリケーションサイズに応じて
max_files = 20000  # ファイル数に応じて
validate_timestamps = false  # 本番環境では無効化
```

## セキュリティハードニング

### SELinuxの設定

```bash
# SELinuxの状態確認
getenforce

# ポリシーの設定
sudo semanage port -a -t http_port_t -p tcp 8080
sudo semanage fcontext -a -t httpd_sys_content_t "/var/www/html(/.*)?"
sudo restorecon -R /var/www/html
```

### ファイアウォール設定

```bash
# UFW（Ubuntu/Debian）
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw allow 9090/tcp  # Prometheus（内部ネットワークのみ推奨）
sudo ufw enable

# firewalld（CentOS/RHEL）
sudo firewall-cmd --permanent --add-service=http
sudo firewall-cmd --permanent --add-service=https
sudo firewall-cmd --permanent --add-port=9090/tcp
sudo firewall-cmd --reload
```

### 定期的なセキュリティアップデート

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get upgrade

# CentOS/RHEL
sudo yum update
```

cron設定（毎週日曜日午前2時）:
```bash
0 2 * * 0 apt-get update && apt-get upgrade -y
```
