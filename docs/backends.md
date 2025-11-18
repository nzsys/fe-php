# Backends

fe-phpは3つの異なるバックエンドをサポートしており、リクエストパスに応じて最適なバックエンドを自動選択するハイブリッドアーキテクチャを採用しています。

## バックエンドの概要

| バックエンド | 実行方法 | 速度 | メモリ | 安定性 | 用途 |
|------------|---------|------|--------|--------|------|
| **Embedded** | libphp直接実行 | 最高速 | 共有 | 中 | API、軽量処理 |
| **FastCGI** | PHP-FPMプロキシ | 高速 | 分離 | 高 | 管理画面、長時間処理 |
| **Static** | Rust直接配信 | 超高速 | 最小 | 最高 | 画像、CSS、JavaScript |

## Embeddedバックエンド

### 概要

libphpをプロセス内で直接実行するバックエンドです。最も高速ですが、PHPとサーバープロセスがメモリを共有するため、PHPのメモリリークやクラッシュがサーバー全体に影響する可能性があります。

### アーキテクチャ

```
┌─────────────────────────────┐
│      fe-php Process         │
│                             │
│  ┌───────────┐              │
│  │  Rust     │              │
│  │  Server   │              │
│  └─────┬─────┘              │
│        │                    │
│        │ (in-process call)  │
│        ▼                    │
│  ┌───────────┐              │
│  │  libphp   │              │
│  │  (ZTS)    │              │
│  └───────────┘              │
└─────────────────────────────┘
```

### パフォーマンス特性

- **レイテンシ**: 1-3ms（最小）
- **スループット**: 最大2.14倍高速（対Nginx + PHP-FPM）
- **メモリ**: プロセス共有のため効率的
- **並列性**: ZTS（Zend Thread Safety）により複数リクエストを並列処理可能

### 設定

```toml
[php]
libphp_path = "/usr/local/php-zts-embed/lib/libphp.so"
document_root = "/var/www/html"
worker_pool_size = 8
worker_max_requests = 10000

[php.opcache]
enable = true
memory_size = "256M"
max_files = 10000
validate_timestamps = false

[backend]
default_backend = "embedded"
```

### 適切なユースケース

- APIエンドポイント（低レイテンシ要求）
- 短時間実行のスクリプト（< 1秒）
- 高トラフィックなエンドポイント（> 100 RPS）
- メモリ使用量が予測可能なアプリケーション

### 避けるべきケース

- 長時間実行のスクリプト（> 30秒）
- メモリリークの可能性があるコード
- サードパーティライブラリの信頼性が低い場合
- クラッシュがサーバー全体に影響することが許容できない場合

## FastCGIバックエンド

### 概要

PHP-FPMにリクエストをプロキシするバックエンドです。プロセス分離により安定性が高く、長時間実行のスクリプトにも対応できます。

### アーキテクチャ

```
┌─────────────────┐         ┌─────────────────┐
│  fe-php Process │         │  PHP-FPM Process│
│                 │         │                 │
│  ┌───────────┐  │         │  ┌───────────┐  │
│  │  Rust     │  │         │  │  PHP      │  │
│  │  Server   │  │         │  │  Worker 1 │  │
│  └─────┬─────┘  │         │  └───────────┘  │
│        │        │         │  ┌───────────┐  │
│        │FastCGI │         │  │  PHP      │  │
│        └────────┼────────▶│  │  Worker 2 │  │
│                 │Protocol │  └───────────┘  │
│                 │         │       ...       │
└─────────────────┘         └─────────────────┘
```

### パフォーマンス特性

- **レイテンシ**: 0-1ms（ローカルソケット使用時）
- **スループット**: 高速（Embeddedより若干遅い）
- **メモリ**: プロセス分離のため独立
- **並列性**: PHP-FPMのワーカー数に依存

### 設定

```toml
[php]
use_fpm = true
fpm_socket = "127.0.0.1:9000"
# または
# fpm_socket = "/run/php/php8.3-fpm.sock"

[backend]
default_backend = "fastcgi"

[backend.connection_pool]
max_size = 50
max_idle_time_secs = 300
max_lifetime_secs = 3600
connect_timeout_secs = 10

[backend.connection_pool.circuit_breaker]
enable = true
failure_threshold = 10
success_threshold = 3
timeout_seconds = 30
```

### 適切なユースケース

- 管理画面（安定性重視）
- 長時間実行のスクリプト（> 30秒）
- メモリ使用量が大きいアプリケーション（> 512MB）
- クラッシュ時の影響を最小化したい場合
- レガシーコードの実行

### 避けるべきケース

- 超低レイテンシが要求される場合（< 5ms）
- 超高トラフィック（> 1000 RPS）でレイテンシを最小化したい場合

## Staticバックエンド

### 概要

RustでファイルシステムからファイルHを直接配信するバックエンドです。PHPのオーバーヘッドが一切なく、最も高速です。

### アーキテクチャ

```
┌─────────────────────────────┐
│      fe-php Process         │
│                             │
│  ┌───────────┐              │
│  │  Rust     │              │
│  │  Server   │              │
│  └─────┬─────┘              │
│        │                    │
│        │ (direct read)      │
│        ▼                    │
│  ┌───────────┐              │
│  │   File    │              │
│  │   System  │              │
│  └───────────┘              │
└─────────────────────────────┘
```

### パフォーマンス特性

- **レイテンシ**: < 1ms（ディスクキャッシュ使用時）
- **スループット**: 最高速
- **メモリ**: 最小（ファイル読み込みのみ）
- **並列性**: 制限なし

### 設定

```toml
[backend.static_files]
enable = true
root = "/var/www/html"
index_files = ["index.html", "index.htm", "default.html"]

[[backend.routing_rules]]
pattern = { type = "prefix", value = "/static/" }
backend = "static"
priority = 100

[[backend.routing_rules]]
pattern = { type = "suffix", value = ".jpg" }
backend = "static"
priority = 90

[[backend.routing_rules]]
pattern = { type = "suffix", value = ".png" }
backend = "static"
priority = 90

[[backend.routing_rules]]
pattern = { type = "suffix", value = ".css" }
backend = "static"
priority = 85

[[backend.routing_rules]]
pattern = { type = "suffix", value = ".js" }
backend = "static"
priority = 85
```

### 適切なユースケース

- 画像ファイル（.jpg, .png, .gif, .svg）
- CSSファイル（.css）
- JavaScriptファイル（.js）
- フォント（.woff, .woff2, .ttf）
- その他の静的アセット
- SPAのindex.html

### 避けるべきケース

- PHPで動的に生成されるコンテンツ
- 認証が必要なファイル（PHPで制御すべき）

## ハイブリッドモード

### 概要

リクエストパスに応じて複数のバックエンドを自動選択するモードです。最適なパフォーマンスと安定性を両立できます。

### 設定

```toml
[backend]
enable_hybrid = true
default_backend = "embedded"

# 優先度が高い順に評価
# 優先度100: 静的ファイルディレクトリ
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/static/" }
backend = "static"
priority = 100

# 優先度90: 画像ファイル
[[backend.routing_rules]]
pattern = { type = "suffix", value = ".jpg" }
backend = "static"
priority = 90

[[backend.routing_rules]]
pattern = { type = "suffix", value = ".png" }
backend = "static"
priority = 90

# 優先度85: CSS/JavaScript
[[backend.routing_rules]]
pattern = { type = "suffix", value = ".css" }
backend = "static"
priority = 85

[[backend.routing_rules]]
pattern = { type = "suffix", value = ".js" }
backend = "static"
priority = 85

# 優先度80: APIエンドポイント → 高速Embedded
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/api/" }
backend = "embedded"
priority = 80

# 優先度70: 管理画面 → 安定性重視FastCGI
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/admin/" }
backend = "fastcgi"
priority = 70

# その他は default_backend (embedded) が使用される
```

### ルーティング評価順序

1. 優先度（`priority`）が高い順に評価
2. 最初にマッチしたルールが適用される
3. どのルールにもマッチしない場合は`default_backend`が使用される

### パターンマッチング

#### プレフィックスマッチ（prefix）

URIが指定された文字列で始まる場合にマッチ。

```toml
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/api/" }
backend = "embedded"
priority = 80
```

マッチ例:
- `/api/users` → マッチ
- `/api/v1/products` → マッチ
- `/users/api` → マッチしない

#### サフィックスマッチ（suffix）

URIが指定された文字列で終わる場合にマッチ。

```toml
[[backend.routing_rules]]
pattern = { type = "suffix", value = ".jpg" }
backend = "static"
priority = 90
```

マッチ例:
- `/images/photo.jpg` → マッチ
- `/photo.jpg` → マッチ
- `/photo.jpeg` → マッチしない

#### 完全一致（exact）

URIが指定された文字列と完全に一致する場合にマッチ。

```toml
[[backend.routing_rules]]
pattern = { type = "exact", value = "/status" }
backend = "embedded"
priority = 100
```

マッチ例:
- `/status` → マッチ
- `/status/` → マッチしない
- `/api/status` → マッチしない

#### 正規表現マッチ（regex）

URIが正規表現とマッチする場合にマッチ。

```toml
[[backend.routing_rules]]
pattern = { type = "regex", value = "^/user/[0-9]+$" }
backend = "embedded"
priority = 70
```

マッチ例:
- `/user/123` → マッチ
- `/user/456` → マッチ
- `/user/abc` → マッチしない
- `/user/123/profile` → マッチしない

## ユースケース別の推奨設定

### SaaS/Webアプリケーション

```toml
[backend]
enable_hybrid = true
default_backend = "embedded"

# 静的アセット
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/assets/" }
backend = "static"
priority = 100

# APIエンドポイント
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/api/" }
backend = "embedded"
priority = 90

# 管理画面
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/admin/" }
backend = "fastcgi"
priority = 80
```

### WordPress

```toml
[backend]
enable_hybrid = true
default_backend = "embedded"

# アップロードファイル
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/wp-content/uploads/" }
backend = "static"
priority = 100

# 静的アセット
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/wp-content/themes/" }
backend = "static"
priority = 95

[[backend.routing_rules]]
pattern = { type = "prefix", value = "/wp-includes/" }
backend = "static"
priority = 95

# 管理画面
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/wp-admin/" }
backend = "fastcgi"
priority = 90

# wp-login.php
[[backend.routing_rules]]
pattern = { type = "exact", value = "/wp-login.php" }
backend = "fastcgi"
priority = 85
```

### ECサイト

```toml
[backend]
enable_hybrid = true
default_backend = "embedded"

# 商品画像
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/images/products/" }
backend = "static"
priority = 100

# 静的アセット
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/static/" }
backend = "static"
priority = 95

# 商品API
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/api/products/" }
backend = "embedded"
priority = 90

# カートAPI
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/api/cart/" }
backend = "embedded"
priority = 90

# 決済処理（安定性重視）
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/checkout/" }
backend = "fastcgi"
priority = 85

# レポート生成（長時間処理）
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/admin/reports/" }
backend = "fastcgi"
priority = 80
```

### SPA（Single Page Application）

```toml
[backend]
enable_hybrid = true
default_backend = "static"

[backend.static_files]
enable = true
root = "/var/www/html/dist"
index_files = ["index.html"]

# 静的アセット
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/assets/" }
backend = "static"
priority = 100

# APIエンドポイント
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/api/" }
backend = "embedded"
priority = 90

# その他は全てindex.htmlにフォールバック（default_backend = "static"）
```

## パフォーマンスチューニング

### Embeddedバックエンド

```toml
[php]
worker_pool_size = 16  # CPU数 × 2 が目安
worker_max_requests = 10000  # メモリリーク対策

[php.opcache]
enable = true
memory_size = "512M"  # アプリケーションサイズに応じて調整
max_files = 20000
validate_timestamps = false  # 本番環境では無効化
```

### FastCGIバックエンド

```toml
[backend.connection_pool]
max_size = 100  # PHP-FPMのワーカー数に応じて調整
max_idle_time_secs = 600  # アイドル接続の保持時間を長めに
connect_timeout_secs = 5  # タイムアウトを短めに
```

PHP-FPM側（`/etc/php-fpm.d/www.conf`）:

```ini
pm = dynamic
pm.max_children = 100
pm.start_servers = 20
pm.min_spare_servers = 10
pm.max_spare_servers = 30
```

### Staticバックエンド

```toml
[backend.static_files]
enable = true
root = "/var/www/html"
index_files = ["index.html", "index.htm"]
```

OSレベルのチューニング:
- ファイルシステムキャッシュの最大化
- `vm.vfs_cache_pressure`の調整（Linux）
- SSDの使用推奨

## トラブルシューティング

### Embeddedバックエンドのメモリリーク

症状: サーバーのメモリ使用量が時間とともに増加

解決方法:
```toml
[php]
worker_max_requests = 5000  # より頻繁にワーカーを再起動
```

### FastCGI接続エラー

症状: `Connection refused` または `Connection timeout`

解決方法:
1. PHP-FPMが起動していることを確認
2. ソケットパスが正しいことを確認
3. 接続プールサイズを調整:
```toml
[backend.connection_pool]
max_size = 200
connect_timeout_secs = 10
```

### Static バックエンドで404エラー

症状: 存在するファイルが見つからない

解決方法:
1. `root`パスが正しいことを確認
2. ファイルパーミッションを確認
3. URIパスとファイルシステムパスの対応を確認:
   - リクエスト: `/static/index.html`
   - ファイル: `{root}/static/index.html`
