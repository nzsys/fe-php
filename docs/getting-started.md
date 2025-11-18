# Getting Started

このガイドでは、fe-phpのインストールから基本的な使い方まで説明します。

## 必要要件

### システム要件

- Linux、macOS、またはその他のUnix系OS
- Rust 1.75以上
- PHP 8.0以上（ZTS版、embed SAPI有効化）

### PHP ZTS（Zend Thread Safety）のインストール

fe-phpのEmbeddedバックエンドを使用するには、ZTS版のPHPが必要です。

#### Linux (Ubuntu/Debian)

```bash
# ビルド依存関係のインストール
sudo apt-get update
sudo apt-get install -y build-essential autoconf libtool bison re2c \
    libxml2-dev libsqlite3-dev libssl-dev libcurl4-openssl-dev \
    libpng-dev libjpeg-dev libonig-dev libzip-dev

# PHPソースコードの取得
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
```

#### macOS (Homebrew使用)

```bash
# Homebrewでインストール
brew install php@8.3

# ZTS版を手動ビルド
cd /tmp
curl -O https://www.php.net/distributions/php-8.3.0.tar.gz
tar -xzf php-8.3.0.tar.gz
cd php-8.3.0

./configure \
    --prefix=/usr/local/php-zts-embed \
    --enable-embed=shared \
    --enable-zts \
    --with-openssl=$(brew --prefix openssl) \
    --with-curl \
    --with-zlib \
    --enable-mbstring \
    --enable-opcache

make -j$(sysctl -n hw.ncpu)
sudo make install
```

### PHP-FPM（FastCGIバックエンド用、オプション）

FastCGIバックエンドを使用する場合は、PHP-FPMが必要です。

#### Linux

```bash
sudo apt-get install php8.3-fpm
sudo systemctl start php8.3-fpm
```

#### macOS

```bash
brew install php
brew services start php
```

## fe-phpのインストール

### ソースからビルド

```bash
# リポジトリのクローン
git clone https://github.com/nzsys/fe-php.git
cd fe-php

# リリースビルド
cargo build --release

# バイナリのインストール（オプション）
sudo cp target/release/fe-php /usr/local/bin/
```

ビルドには数分かかります。完了後、`target/release/fe-php`にバイナリが生成されます。

## 初期設定

### 最小設定ファイルの作成

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

[metrics]
enable = true
endpoint = "/_metrics"
port = 9090

[logging]
level = "info"
format = "json"
output = "stdout"
```

### ドキュメントルートの作成

```bash
sudo mkdir -p /var/www/html
```

### テスト用PHPファイルの作成

`/var/www/html/index.php`:

```php
<?php
phpinfo();
```

`/var/www/html/api/test.php`:

```php
<?php
header('Content-Type: application/json');
echo json_encode([
    'status' => 'ok',
    'timestamp' => time(),
    'server' => 'fe-php'
]);
```

## サーバーの起動

```bash
fe-php serve --config config.toml
```

サーバーが起動すると、以下のようなログが表示されます：

```
{"timestamp":"2025-11-18T00:00:00.000Z","level":"INFO","message":"Server starting","host":"0.0.0.0","port":8080}
{"timestamp":"2025-11-18T00:00:00.100Z","level":"INFO","message":"Worker pool initialized","workers":8}
{"timestamp":"2025-11-18T00:00:00.200Z","level":"INFO","message":"Server started successfully"}
```

## 動作確認

### ヘルスチェック

```bash
curl http://localhost:8080/_health
```

期待される出力:
```json
{
  "status": "healthy",
  "uptime_seconds": 10,
  "version": "0.1.0"
}
```

### PHPファイルの実行

```bash
curl http://localhost:8080/index.php
```

PHPの情報ページ（phpinfo）が表示されます。

### APIエンドポイントのテスト

```bash
curl http://localhost:8080/api/test.php
```

期待される出力:
```json
{
  "status": "ok",
  "timestamp": 1700000000,
  "server": "fe-php"
}
```

### メトリクスの確認

```bash
curl http://localhost:9090/_metrics
```

Prometheus形式のメトリクスが表示されます。

### Admin APIの確認

```bash
curl http://localhost:9001/api/status | jq .
```

期待される出力:
```json
{
  "server": {
    "version": "0.1.0",
    "uptime_seconds": 10,
    "pid": 12345,
    "started_at": 1700000000
  },
  "metrics": {
    "requests_per_second": 0.0,
    "active_connections": 0,
    "total_requests": 0,
    "error_rate": 0.0
  },
  "backends": [
    {
      "name": "Embedded (libphp)",
      "backend_type": "embedded",
      "status": "healthy",
      "requests": 0,
      "errors": 0,
      "avg_response_ms": 0.0
    }
  ]
}
```

## TUI Monitor の起動

サーバーが起動している状態で、別のターミナルでMonitorを起動します：

```bash
fe-php monitor --socket /var/run/fe-php-admin.sock
```

Monitorが起動すると、ターミナルUIが表示されます。タブキーで各タブを切り替えられます：

- **Overview**: サーバー概要とメトリクス
- **Metrics**: 詳細メトリクス
- **Backends**: バックエンド別統計
- **Security**: ブロック済みIP一覧
- **Logs**: リクエストログ
- **Analysis**: ログ分析結果
- **Help**: ヘルプ

キーボード操作：
- `Tab` / `Shift+Tab`: タブ切り替え
- `↑` / `↓`: スクロール
- `r`: 手動リフレッシュ
- `q`: 終了

## トラブルシューティング

### libphpが見つからない

```
Error: Failed to load libphp: cannot open shared object file
```

解決方法：
1. libphpのパスを確認：`find /usr/local -name "libphp.so"`
2. 設定ファイルの`libphp_path`を正しいパスに修正
3. `LD_LIBRARY_PATH`環境変数を設定（Linux）：
   ```bash
   export LD_LIBRARY_PATH=/usr/local/php-zts-embed/lib:$LD_LIBRARY_PATH
   ```

### Unix Socketの権限エラー

```
Error: Permission denied (os error 13)
```

解決方法：
1. Unix Socketのディレクトリが存在することを確認：`sudo mkdir -p /var/run`
2. 権限を変更：`sudo chmod 755 /var/run`
3. または設定ファイルで別のパスを指定：
   ```toml
   [admin]
   unix_socket = "/tmp/fe-php-admin.sock"
   ```

### ポートが既に使用されている

```
Error: Address already in use (os error 48)
```

解決方法：
1. 使用中のポートを確認：`lsof -i :8080`
2. 設定ファイルで別のポートを指定：
   ```toml
   [server]
   port = 8081
   ```

### PHP-FPMに接続できない

```
Error: Connection refused
```

解決方法：
1. PHP-FPMが起動していることを確認：
   - Linux: `sudo systemctl status php8.3-fpm`
   - macOS: `brew services list`
2. PHP-FPMのソケットパスを確認：
   - Linux: `/run/php/php8.3-fpm.sock` または `127.0.0.1:9000`
   - macOS: `127.0.0.1:9000`
3. 設定ファイルを修正：
   ```toml
   [php]
   fpm_socket = "127.0.0.1:9000"
   # または
   # fpm_socket = "/run/php/php8.3-fpm.sock"
   ```

## 次のステップ

- [Configuration](configuration.md) - 詳細な設定オプション
- [Backends](backends.md) - バックエンドの選択と設定
- [Monitoring](monitoring.md) - 監視とメトリクス
- [Security](security.md) - セキュリティ機能の設定
- [Deployment](deployment.md) - 本番環境へのデプロイ
