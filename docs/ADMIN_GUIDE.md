# Admin Console ガイド

fe-phpには、サーバーの状態を監視・管理するためのAdmin Consoleが組み込まれています。

## 機能概要

### Web Dashboard
- サーバー状態の可視化
- リアルタイムメトリクス表示
- バックエンド状態の監視
- 日本語対応UI

### JSON API
- プログラマブルなアクセス
- モニタリングツールとの連携
- 自動化スクリプト対応

## 設定方法

### 基本設定

`config.toml`に以下を追加：

```toml
[admin]
enable = true              # Admin機能を有効化
host = "127.0.0.1"        # リスニングアドレス（セキュリティのため127.0.0.1推奨）
http_port = 9000          # HTTPポート
unix_socket = "/var/run/fe-php.sock"  # Unix socket（オプション）
allowed_ips = ["127.0.0.1"]  # 許可するIPアドレスのリスト
```

### 設定項目の説明

| 項目 | 説明 | デフォルト | 推奨値 |
|------|------|-----------|--------|
| `enable` | Admin機能の有効/無効 | `false` | `true` |
| `host` | リスニングホスト | `"127.0.0.1"` | `"127.0.0.1"` (本番環境) |
| `http_port` | HTTPポート番号 | `9000` | `9000` |
| `unix_socket` | Unix socketパス | - | `/var/run/fe-php-admin.sock` |
| `allowed_ips` | 許可IPリスト | `["127.0.0.1"]` | 環境に応じて設定 |

## 起動方法

### 1. サーバーを起動

```bash
./target/release/fe-php serve examples/hybrid_backend_config.toml
```

起動時のログで確認：
```
Admin interface available at http://127.0.0.1:9000
```

### 2. Admin Consoleにアクセス

ブラウザで以下のURLにアクセス：

```
http://127.0.0.1:9000/
```

## 使用方法

### Webダッシュボード

#### メインページ (`/`)

1. **サーバー状態**
   - Uptime: サーバーの稼働時間
   - Started At: 起動日時
   - Version: fe-phpのバージョン
   - Process ID: プロセスID

2. **リアルタイムメトリクス**
   - Requests/sec: 秒間リクエスト数
   - Active Connections: アクティブな接続数
   - Total Requests: 累計リクエスト数
   - Error Rate: エラー率

3. **バックエンド状態**
   - Name: バックエンド名
   - Type: バックエンドタイプ（embedded/fastcgi/static）
   - Status: 状態（healthy/degraded/down）
   - Requests: リクエスト数
   - Errors: エラー数
   - Avg Response: 平均レスポンス時間

#### ナビゲーション

- **Dashboard**: メインダッシュボード（現在のページ）
- **Metrics**: Prometheusメトリクス（実装予定）
- **Logs**: ログビューア（実装予定）
- **WAF**: WAF統計情報（実装予定）
- **Backends**: バックエンド詳細（実装予定）
- **System**: システム情報（実装予定）
- **JSON API**: API仕様

### JSON API

#### エンドポイント一覧

##### 1. サーバー状態取得

```bash
curl http://127.0.0.1:9000/api/status
```

レスポンス例：
```json
{
  "server": {
    "version": "0.1.0",
    "uptime_seconds": 3600,
    "pid": 12345,
    "started_at": 1705234567
  },
  "metrics": {
    "requests_per_second": 123.45,
    "active_connections": 42,
    "total_requests": 444600,
    "error_rate": 0.05
  },
  "backends": [
    {
      "name": "embedded",
      "backend_type": "embedded",
      "status": "healthy",
      "requests": 400000,
      "errors": 20,
      "avg_response_ms": 1.23
    }
  ]
}
```

#### curlでの使用例

```bash
# 基本的な状態取得
curl http://127.0.0.1:9000/api/status

# 整形して表示（jqを使用）
curl -s http://127.0.0.1:9000/api/status | jq .

# uptimeのみ取得
curl -s http://127.0.0.1:9000/api/status | jq '.server.uptime_seconds'

# リクエスト数を取得
curl -s http://127.0.0.1:9000/api/status | jq '.metrics.total_requests'
```

## セキュリティ

### 推奨設定

1. **ローカルホストのみ許可**
   ```toml
   [admin]
   host = "127.0.0.1"  # 外部からアクセス不可
   allowed_ips = ["127.0.0.1"]
   ```

2. **リバースプロキシ経由でアクセス**

   Nginxの設定例：
   ```nginx
   server {
       listen 80;
       server_name admin.example.com;

       # Basic認証
       auth_basic "Admin Console";
       auth_basic_user_file /etc/nginx/.htpasswd;

       # SSL推奨
       # listen 443 ssl;
       # ssl_certificate /path/to/cert.pem;
       # ssl_certificate_key /path/to/key.pem;

       location / {
           proxy_pass http://127.0.0.1:9000;
           proxy_set_header Host $host;
           proxy_set_header X-Real-IP $remote_addr;
       }
   }
   ```

3. **ファイアウォール設定**
   ```bash
   # iptablesでポート9000を保護
   sudo iptables -A INPUT -p tcp --dport 9000 -s 127.0.0.1 -j ACCEPT
   sudo iptables -A INPUT -p tcp --dport 9000 -j DROP
   ```

### セキュリティチェックリスト

- [ ] `host`を`127.0.0.1`に設定
- [ ] `allowed_ips`で許可IPを制限
- [ ] リバースプロキシでBasic認証を設定
- [ ] HTTPS/TLSを有効化（本番環境）
- [ ] ファイアウォールで直接アクセスを制限
- [ ] アクセスログを監視

## 📈 モニタリング連携

### Prometheusとの連携

Admin ConsoleとPrometheusメトリクスを併用：

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'fe-php'
    static_configs:
      - targets: ['localhost:9090']  # メトリクスエンドポイント
    scrape_interval: 15s
```

### Grafanaダッシュボード

1. Prometheusをデータソースに追加
2. Admin API(`/api/status`)をJSON APIデータソースに追加
3. 両方のデータを可視化

### 監視スクリプト例

```bash
#!/bin/bash
# check_fe-php.sh - Admin APIを使った監視スクリプト

API="http://127.0.0.1:9000/api/status"

# エラー率チェック
ERROR_RATE=$(curl -s $API | jq -r '.metrics.error_rate')
if (( $(echo "$ERROR_RATE > 5.0" | bc -l) )); then
    echo "CRITICAL: Error rate is ${ERROR_RATE}%"
    exit 2
fi

# アクティブ接続数チェック
ACTIVE=$(curl -s $API | jq -r '.metrics.active_connections')
if [ "$ACTIVE" -gt 1000 ]; then
    echo "WARNING: High active connections: $ACTIVE"
    exit 1
fi

echo "OK: Server healthy"
exit 0
```

## トラブルシューティング

### Admin Consoleにアクセスできない

1. **設定を確認**
   ```bash
   grep -A 5 "\[admin\]" config.toml
   ```

2. **ポートが使用されているか確認**
   ```bash
   lsof -i :9000
   # または
   netstat -an | grep 9000
   ```

3. **ログを確認**
   ```bash
   # サーバー起動時のログ
   ./target/release/fe-php serve config.toml 2>&1 | grep -i admin
   ```

4. **ファイアウォールを確認**
   ```bash
   sudo iptables -L -n | grep 9000
   ```

### 「Connection refused」エラー

```bash
# サーバーが起動しているか確認
ps aux | grep fe-php

# 正しいポートで待ち受けているか確認
netstat -tlnp | grep fe-php
```

### JSON APIが404を返す

- URLを確認：`/api/status`（先頭に`/`が必要）
- curlで詳細を確認：
  ```bash
  curl -v http://127.0.0.1:9000/api/status
  ```

## 🔮 今後の機能追加予定

- [ ] リアルタイムログビューア
- [ ] WAF統計情報とルール管理
- [ ] バックエンドの手動有効/無効切り替え
- [ ] 設定のホットリロード
- [ ] ワーカープールの再起動
- [ ] パフォーマンスグラフの表示
- [ ] アラート設定機能

## ℹ️ サポート

問題が解決しない場合は、GitHubのIssueで報告してください。
