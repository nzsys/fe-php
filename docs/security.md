# Security

fe-phpは複数のセキュリティ機能を提供しています。

## WAF (Web Application Firewall)

リクエストを検査し、悪意のあるパターンを検出・ブロックします。

### 設定

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

### パラメータ

| パラメータ | 型 | デフォルト | 説明 |
|----------|-------|----------|------|
| `enable` | boolean | `false` | WAFを有効化 |
| `mode` | string | `"detect"` | 動作モード（`detect`: ログのみ、`block`: ブロック） |
| `rules_path` | string | - | WAFルールファイルのパス |

### WAFルールファイル

`waf_rules.toml`:

```toml
[[rules]]
id = "SQL_INJECTION"
pattern = "(?i)(union|select|insert|update|delete|drop|create|alter)\\s+"
severity = "high"
action = "block"
description = "SQL injection attempt detected"

[[rules]]
id = "XSS"
pattern = "(?i)<script|javascript:|onerror=|onload=|eval\\(|expression\\("
severity = "high"
action = "block"
description = "Cross-site scripting attempt detected"

[[rules]]
id = "PATH_TRAVERSAL"
pattern = "\\.\\./|\\.\\.\\\\"
severity = "high"
action = "block"
description = "Path traversal attempt detected"

[[rules]]
id = "COMMAND_INJECTION"
pattern = "(?i)(;\\s*(cat|ls|wget|curl|nc|bash|sh|perl|python|ruby))"
severity = "high"
action = "block"
description = "Command injection attempt detected"

[[rules]]
id = "FILE_INCLUSION"
pattern = "(?i)(include|require)\\s*\\(.*\\$"
severity = "medium"
action = "detect"
description = "Potential file inclusion vulnerability"

[[rules]]
id = "SENSITIVE_FILES"
pattern = "(?i)(\\.(env|git|svn|htaccess|htpasswd)|/\\.)"
severity = "medium"
action = "block"
description = "Attempt to access sensitive files"
```

### ルールパラメータ

| フィールド | 型 | 説明 |
|----------|-------|------|
| `id` | string | ルールID（一意） |
| `pattern` | string | マッチさせる正規表現パターン |
| `severity` | string | 重要度（`low`, `medium`, `high`, `critical`） |
| `action` | string | アクション（`detect`: ログのみ、`block`: ブロック） |
| `description` | string | ルールの説明 |

### WAFログ

WAFが検出・ブロックした場合、以下のようなログが出力されます：

```json
{
  "timestamp": "2025-11-18T12:34:56.789Z",
  "level": "warn",
  "message": "WAF rule triggered",
  "rule_id": "SQL_INJECTION",
  "severity": "high",
  "action": "block",
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "method": "GET",
  "uri": "/search?q=1' UNION SELECT * FROM users--",
  "remote_addr": "192.168.1.100",
  "user_agent": "Mozilla/5.0..."
}
```

### WAFのチューニング

#### 検出モード（開発時）

まず検出モードで誤検知を確認：

```toml
[waf]
mode = "detect"
```

ログを確認して誤検知がある場合、ルールを調整します。

#### ブロックモード（本番環境）

誤検知がないことを確認後、ブロックモードに切り替え：

```toml
[waf]
mode = "block"
```

#### カスタムルールの追加

アプリケーション固有の脅威に対してカスタムルールを追加：

```toml
[[rules]]
id = "CUSTOM_ADMIN_ACCESS"
pattern = "^/admin/.*\\?.*<script"
severity = "critical"
action = "block"
description = "XSS attempt in admin area"
```

## レート制限

IP別にリクエストレートを制限します。

### 設定

```toml
[waf.rate_limit]
requests_per_ip = 100
window_seconds = 60
burst = 20
```

### パラメータ

| パラメータ | 型 | デフォルト | 説明 |
|----------|-------|----------|------|
| `requests_per_ip` | integer | `100` | IPあたりの最大リクエスト数 |
| `window_seconds` | integer | `60` | レート制限のウィンドウサイズ（秒） |
| `burst` | integer | `20` | 一時的に許可する最大バースト |

### 動作

1. 各IPアドレスのリクエスト数をカウント
2. `window_seconds`秒間で`requests_per_ip`を超えた場合、429 Too Many Requestsを返す
3. `burst`までは一時的に超過を許可

例: `requests_per_ip = 100`, `window_seconds = 60`, `burst = 20`の場合:
- 通常: 60秒間に100リクエストまで許可
- バースト: 一時的に120リクエストまで許可

### レート制限のログ

```json
{
  "timestamp": "2025-11-18T12:34:56.789Z",
  "level": "warn",
  "message": "Rate limit exceeded",
  "remote_addr": "192.168.1.100",
  "requests_count": 125,
  "limit": 100,
  "window_seconds": 60
}
```

### レート制限のチューニング

#### API重視の設定

```toml
[waf.rate_limit]
requests_per_ip = 1000  # 高いレート制限
window_seconds = 60
burst = 200
```

#### セキュリティ重視の設定

```toml
[waf.rate_limit]
requests_per_ip = 50  # 厳しいレート制限
window_seconds = 60
burst = 10
```

## IPフィルタリング

IPアドレスベースのアクセス制御を提供します。

### Admin APIのIP制限

```toml
[admin]
enable = true
host = "127.0.0.1"
http_port = 9001
allowed_ips = ["127.0.0.1", "::1", "10.0.0.0/8", "192.168.1.0/24"]
```

CIDR表記をサポート：
- `127.0.0.1`: ローカルホストのみ
- `10.0.0.0/8`: 10.0.0.0 ～ 10.255.255.255
- `192.168.1.0/24`: 192.168.1.0 ～ 192.168.1.255
- `::1`: IPv6ローカルホスト
- `2001:db8::/32`: IPv6ネットワーク

### GeoIPフィルタリング

国別のアクセス制御を提供します。

#### セットアップ

1. MaxMind GeoIPデータベースの取得：

```bash
# GeoLite2データベースのダウンロード（無料）
wget https://github.com/P3TERX/GeoLite.mmdb/raw/download/GeoLite2-Country.mmdb
sudo mkdir -p /usr/share/GeoIP
sudo mv GeoLite2-Country.mmdb /usr/share/GeoIP/
```

2. 設定ファイルの編集：

```toml
[geoip]
enable = true
database_path = "/usr/share/GeoIP/GeoLite2-Country.mmdb"
allowed_countries = ["JP", "US", "GB"]  # 許可する国（ISO 3166-1 alpha-2）
blocked_countries = ["CN", "RU"]  # ブロックする国（優先）
```

#### 国コード（ISO 3166-1 alpha-2）

主要な国コード：
- `JP`: 日本
- `US`: アメリカ
- `GB`: イギリス
- `DE`: ドイツ
- `FR`: フランス
- `CN`: 中国
- `RU`: ロシア
- `KR`: 韓国
- `AU`: オーストラリア
- `CA`: カナダ

#### ホワイトリストモード

特定の国のみ許可：

```toml
[geoip]
allowed_countries = ["JP"]  # 日本のみ
blocked_countries = []
```

#### ブラックリストモード

特定の国をブロック：

```toml
[geoip]
allowed_countries = []  # 全て許可
blocked_countries = ["CN", "RU"]  # 中国とロシアをブロック
```

#### 組み合わせ

`blocked_countries`が`allowed_countries`より優先されます：

```toml
[geoip]
allowed_countries = ["JP", "US", "GB"]
blocked_countries = ["US"]  # USはブロックされる
# 結果: JPとGBのみ許可
```

## 動的IPブロック

ランタイムでIPアドレスをブロック・解除できます。

### Admin API経由でのブロック

#### HTTP API

```bash
# IPブロック
curl -X POST http://localhost:9001/api/security/block-ip \
  -H "Content-Type: application/json" \
  -d '{"ip":"192.168.1.100"}'

# IPブロック解除
curl -X POST http://localhost:9001/api/security/unblock-ip \
  -H "Content-Type: application/json" \
  -d '{"ip":"192.168.1.100"}'

# ブロック済みIP一覧
curl http://localhost:9001/api/security/blocked-ips
```

#### Unix Socket

```bash
# IPブロック
echo '{"command":"block_ip","ip":"192.168.1.100"}' | \
  socat - UNIX-CONNECT:/var/run/fe-php-admin.sock

# IPブロック解除
echo '{"command":"unblock_ip","ip":"192.168.1.100"}' | \
  socat - UNIX-CONNECT:/var/run/fe-php-admin.sock
```

### TUI Monitor経由でのブロック

TUI MonitorのSecurityタブから不審なIPを確認し、Analysisタブでスキャン行為を検出した場合、Admin API経由で手動ブロックできます。

### 自動ブロックスクリプト

ログを監視して自動的にIPをブロックするスクリプト例：

```bash
#!/bin/bash
# auto_block.sh - 不審なアクティビティを検出してIPをブロック

SOCKET="/var/run/fe-php-admin.sock"
THRESHOLD_404=20  # 404エラーの閾値
THRESHOLD_5XX=10  # 5xxエラーの閾値

while true; do
  # ログ分析結果を取得
  ANALYSIS=$(echo '{"command":"status"}' | socat - UNIX-CONNECT:$SOCKET 2>/dev/null | \
    jq -r '.server_status.recent_logs')

  # 404エラーが多いIPを抽出
  SCAN_IPS=$(echo "$ANALYSIS" | jq -r \
    "group_by(.remote_addr) | map(select(map(select(.status == 404)) | length > $THRESHOLD_404) | .[0].remote_addr) | .[]")

  # IPをブロック
  for IP in $SCAN_IPS; do
    echo "Blocking IP: $IP (excessive 404 errors)"
    echo "{\"command\":\"block_ip\",\"ip\":\"$IP\"}" | \
      socat - UNIX-CONNECT:$SOCKET
  done

  sleep 60
done
```

## TLS/SSL

HTTPS通信を暗号化します。

### 設定

```toml
[tls]
enable = true
cert_path = "/etc/ssl/certs/server.crt"
key_path = "/etc/ssl/private/server.key"
ca_cert_path = "/etc/ssl/certs/ca.crt"  # クライアント認証用（オプション）
alpn_protocols = ["h2", "http/1.1"]
http_redirect = true  # HTTPをHTTPSにリダイレクト
http_port = 80
```

### Let's Encryptの使用

#### Certbotでの証明書取得

```bash
# Certbotのインストール
sudo apt-get install certbot

# スタンドアロンモードで証明書取得
sudo certbot certonly --standalone -d example.com

# 証明書のパス
# cert_path: /etc/letsencrypt/live/example.com/fullchain.pem
# key_path: /etc/letsencrypt/live/example.com/privkey.pem
```

#### 設定

```toml
[tls]
enable = true
cert_path = "/etc/letsencrypt/live/example.com/fullchain.pem"
key_path = "/etc/letsencrypt/live/example.com/privkey.pem"
alpn_protocols = ["h2", "http/1.1"]
http_redirect = true
http_port = 80
```

#### 証明書の自動更新

```bash
# cron設定（毎日実行、証明書が期限切れ間近なら更新）
0 0 * * * certbot renew --quiet --post-hook "systemctl reload fe-php"
```

### 自己署名証明書（開発用）

```bash
# 自己署名証明書の生成
openssl req -x509 -newkey rsa:4096 -nodes \
  -keyout /tmp/server.key \
  -out /tmp/server.crt \
  -days 365 \
  -subj "/CN=localhost"

# 設定
[tls]
enable = true
cert_path = "/tmp/server.crt"
key_path = "/tmp/server.key"
alpn_protocols = ["h2", "http/1.1"]
```

### クライアント証明書認証

クライアント証明書による認証を有効化：

```toml
[tls]
enable = true
cert_path = "/etc/ssl/certs/server.crt"
key_path = "/etc/ssl/private/server.key"
ca_cert_path = "/etc/ssl/certs/ca.crt"  # クライアント証明書を検証するCA証明書
```

## セキュリティのベストプラクティス

### 開発環境

- WAFは検出モードで使用
- レート制限は緩めに設定
- ログレベルは`debug`
- Admin APIはローカルのみ許可

```toml
[waf]
enable = true
mode = "detect"

[waf.rate_limit]
requests_per_ip = 1000
window_seconds = 60

[admin]
host = "127.0.0.1"
allowed_ips = ["127.0.0.1"]

[logging]
level = "debug"
```

### 本番環境

- WAFはブロックモードで使用
- レート制限を適切に設定
- ログレベルは`info`または`warn`
- Admin APIはUnix Socketのみ
- TLS/SSLを有効化
- GeoIPフィルタリングを検討

```toml
[waf]
enable = true
mode = "block"
rules_path = "/etc/fe-php/waf_rules.toml"

[waf.rate_limit]
requests_per_ip = 100
window_seconds = 60
burst = 20

[admin]
enable = true
unix_socket = "/var/run/fe-php-admin.sock"
# http_portは設定しない（HTTP API無効化）

[tls]
enable = true
cert_path = "/etc/letsencrypt/live/example.com/fullchain.pem"
key_path = "/etc/letsencrypt/live/example.com/privkey.pem"
http_redirect = true

[geoip]
enable = true
database_path = "/usr/share/GeoIP/GeoLite2-Country.mmdb"
allowed_countries = ["JP", "US"]

[logging]
level = "info"
format = "json"
output = "/var/log/fe-php/access.log"
```

### アラート設定

PrometheusとGrafanaでセキュリティアラートを設定：

```yaml
# alert.rules
groups:
  - name: security
    interval: 30s
    rules:
      - alert: HighWAFTriggerRate
        expr: rate(waf_triggers_total[5m]) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High WAF trigger rate"
          description: "WAF triggered {{ $value }} times per second"

      - alert: RateLimitExceeded
        expr: rate(rate_limit_exceeded_total[5m]) > 5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Rate limit exceeded frequently"
          description: "Rate limit exceeded {{ $value }} times per second"
```

### 監査ログ

セキュリティイベントを別ファイルに記録：

```toml
[logging]
level = "info"
format = "json"
output = "/var/log/fe-php/access.log"

# セキュリティイベント専用のログ設定（アプリケーションレベルで実装が必要）
# security_log = "/var/log/fe-php/security.log"
```

ログのローテーション（logrotate設定例）：

```
/var/log/fe-php/*.log {
    daily
    missingok
    rotate 30
    compress
    delaycompress
    notifempty
    create 0640 www-data www-data
    sharedscripts
    postrotate
        systemctl reload fe-php
    endscript
}
```

### 定期的なセキュリティ監査

1. WAFログの確認（誤検知の有無）
2. レート制限ログの確認（過度な制限の有無）
3. ブロック済みIPの確認（不要なブロックの解除）
4. TLS証明書の有効期限確認
5. GeoIPデータベースの更新（月1回推奨）

```bash
# セキュリティ監査スクリプト例
#!/bin/bash
echo "=== WAF Statistics ==="
cat /var/log/fe-php/access.log | jq -r 'select(.waf_triggered == true) | .rule_id' | sort | uniq -c | sort -rn

echo "=== Rate Limit Exceeded ==="
cat /var/log/fe-php/access.log | jq -r 'select(.message == "Rate limit exceeded") | .remote_addr' | sort | uniq -c | sort -rn

echo "=== Blocked IPs ==="
curl -s http://localhost:9001/api/security/blocked-ips | jq .

echo "=== TLS Certificate Expiry ==="
echo | openssl s_client -servername example.com -connect localhost:443 2>/dev/null | openssl x509 -noout -dates
```
