# fe-php

**ハイブリッドPHPアプリケーションサーバー**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![PHP](https://img.shields.io/badge/php-8.0%2B-777BB4.svg)](https://www.php.net/)

## 概要

**fe-php**は、Rustで実装されたPHPアプリケーションプラットフォームです。**libphp直接実行**、**PHP-FPMプロキシ**、**静的ファイル配信**を単一バイナリで実現し、リクエストパスに応じて最適なバックエンドを自動選択する**ハイブリッドアーキテクチャ**を採用しています。  
fe-phpは、単一のRustバイナリで以下をすべて提供します：

```
従来:                              fe-php:
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
                                  │  • Admin Console     │
                                  └──────────────────────┘
                                  単一バイナリ
                                  統合設定
```

## 特徴

### ハイブリッドバックエンド

3つのバックエンドをリクエストパスに応じて自動選択：

| バックエンド | 速度 | 用途 | 特徴 |
|------------|------|------|------|
| **Embedded** | ⚡⚡⚡ | API、軽量処理 | libphp直接実行（最速） |
| **FastCGI** | ⚡⚡ | 管理画面、長時間処理 | PHP-FPMプロキシ（安定） |
| **Static** | ⚡⚡⚡⚡ | 画像、CSS、JS | Rust直接配信（ゼロPHPオーバーヘッド） |

**グレースフルデグレーデーション**: ハイブリッドモードでlibphpのロードに失敗した場合、自動的にFastCGI専用モードにフォールバックします。

```toml
# 設定例：パスに応じて最適なバックエンドを選択
[[backend.routing_rules]]
pattern = { prefix = "/api/" }
backend = "embedded"      # API → 超高速
priority = 100

[[backend.routing_rules]]
pattern = { suffix = ".jpg" }
backend = "static"        # 画像 → 直接配信
priority = 90

[[backend.routing_rules]]
pattern = { prefix = "/admin/" }
backend = "fastcgi"       # 管理画面 → 安定性重視
priority = 80
```

## パフォーマンス

**テスト環境**: Apple M1 Max、32GB RAM、macOS Sequoia 15.1

###  高並列負荷（500 RPS目標、並列50）

高並列環境でのfe-phpの優位性が明確に現れます：

| サーバー | 実測RPS | Nginx比 | p50レイテンシ | p99レイテンシ |
|---------|--------|---------|-------------|-------------|
| Nginx + PHP-FPM | 209.86 | **1.0x** | 4ms | 10ms |
| fe-php (Embedded) | 358.80 | **1.71x**  | 1ms | 4ms |
| fe-php (FastCGI) | 288.51 | **1.37x** | 0ms | 1ms |
| **fe-php (Hybrid)** | **374.80** | **1.79x**  | **2ms** | **3ms** |

### ストレステスト（1000 RPS目標、並列100）

極限状態でのスループット差がさらに顕著に：

| サーバー | 実測RPS | Nginx比 | p50レイテンシ | p99レイテンシ |
|---------|--------|---------|-------------|-------------|
| Nginx + PHP-FPM | 201.18 | **1.0x** | 4ms | 11ms |
| **fe-php (Embedded)** | **429.76** | **2.14x** | **1ms** | **5ms** |
| fe-php (FastCGI) | 416.26 | **2.07x** | 0ms | 1ms |
| fe-php (Hybrid) | 262.80 | **1.31x** | 3ms | 7ms |

### レイテンシ重視（10 RPS、並列1）

低負荷時のレイテンシ特性：

| サーバー | p50レイテンシ | p99レイテンシ | 改善率 |
|---------|-------------|-------------|--------|
| Nginx + PHP-FPM | 9ms | 18ms | - |
| **fe-php (Embedded)** | **3ms** | **9ms** | **66%改善** |
| fe-php (FastCGI) | 0ms | 1ms | 89%改善 |
| **fe-php (Hybrid)** | **3ms** | **8ms** | **66%改善** |

### 全フェーズ統合結果

| Phase | 負荷条件 | Nginx RPS | fe-php (Hybrid) RPS | 性能比 |
|-------|---------|-----------|-------------------|--------|
| Phase 1 | 50 RPS目標、並列5 | 45.59 | 45.66 | 1.00x |
| Phase 2 | 200 RPS目標、並列20 | 124.59 | 154.70 | **1.24x** |
| Phase 3 | 500 RPS目標、並列50 | 209.86 | 374.80 | **1.79x** |
| Phase 4 | 1000 RPS目標、並列100 | 201.18 | 262.80 | **1.31x** |
| Phase 5 | 10 RPS目標、並列1 | 9.81 | 9.81 | 1.00x |

### パフォーマンスハイライト

- **高並列負荷で最大1.79倍高速**
- **Embeddedモードで最大2.14倍高速**
- **レイテンシ66%改善**
- **並列性能でRust実装の優位性が顕著**
- **低負荷時も安定したパフォーマンス**

### エンタープライズ機能

- **Admin Console** - Webベースの管理画面（リアルタイムメトリクス、バックエンド状態）
- **WAF (Web Application Firewall)** - SQLインジェクション、XSS検知/ブロック
- **Prometheusメトリクス** - バックエンド別の詳細なパフォーマンス測定
- **グレースフルシャットダウン** - SIGTERM/SIGINT対応
- **設定ホットリロード** - SIGUSR1シグナルで無停止設定変更
- **TLS/SSL対応** - Let's Encrypt対応、SNI対応
- **IPフィルタリング** - CIDR表記によるホワイトリスト/ブラックリスト
- **CORS対応** - オリジン/メソッド/ヘッダーの細かい制御

## クイックスタート

### インストール

```bash
# リポジトリをクローン
git clone https://github.com/nzsys/fe-php.git
cd fe-php

# リリースビルド
cargo build --release

# バイナリのインストール
sudo cp target/release/fe-php /usr/local/bin/
```

### 基本的な使い方

```bash
# サーバー起動（デフォルト設定）
fe-php serve

# 設定ファイル指定
fe-php serve --config /path/to/config.toml

# ヘルスチェック
curl http://localhost:8080/_health

# Admin Console（ブラウザで開く）
open http://localhost:9002

# 設定リロード（実行中のサーバーに対して）
kill -SIGUSR1 $(pgrep fe-php)

# ベンチマーク実行
fe-php bench --url http://localhost:8080/bench.php --duration 30 --rps 500 --concurrency 50
```

### 最小設定

```toml
[server]
host = "0.0.0.0"
port = 8080
workers = 4

[php]
libphp_path = "/usr/local/php-zts-embed/lib/libphp.dylib"  # macOS: .dylib, Linux: .so
document_root = "/var/www/html"
php_ini_path = "/etc/php/php.ini"

[backend]
enable_hybrid = true
default_backend = "embedded"

[admin]
enable = true
http_port = 9002
```

### ハイブリッドモード設定例

```toml
[backend]
enable_hybrid = true
default_backend = "embedded"

# APIエンドポイント → 超高速Embedded
[[backend.routing_rules]]
pattern = { prefix = "/api/" }
backend = "embedded"
priority = 100

# 静的ファイル → 直接配信
[[backend.routing_rules]]
pattern = { suffix = ".jpg" }
backend = "static"
priority = 90

[[backend.routing_rules]]
pattern = { suffix = ".png" }
backend = "static"
priority = 90

[[backend.routing_rules]]
pattern = { suffix = ".css" }
backend = "static"
priority = 85

[[backend.routing_rules]]
pattern = { suffix = ".js" }
backend = "static"
priority = 85

# 管理画面 → 安定性重視のFastCGI
[[backend.routing_rules]]
pattern = { prefix = "/admin/" }
backend = "fastcgi"
priority = 70

[backend.static_files]
enable = true
root = "/var/www/html/public"
index_files = ["index.html", "index.htm"]
```

## Admin Console

fe-phpには**Webベースの管理コンソール**が組み込まれています：

- **ダッシュボード** (`http://localhost:9002/`)
  - サーバー情報（バージョン、稼働時間、PID）
  - リアルタイムメトリクス（RPS、接続数、総リクエスト数、エラー率）
  - バックエンド状態テーブル（Embedded, FastCGI, Static）

- **JSON API** (`http://localhost:9002/api/status`)
  - プログラマティックアクセス用API

```toml
# Admin Consoleの有効化
[admin]
enable = true
host = "127.0.0.1"
http_port = 9002
allowed_ips = ["127.0.0.1"]  # セキュリティ設定
```

## ユースケース

### SaaS/Webアプリケーション

```toml
# API → 超高速、静的ファイル → 直接配信、管理画面 → 安定性
default_backend = "embedded"

[[backend.routing_rules]]
pattern = { prefix = "/api/" }
backend = "embedded"
priority = 100

[[backend.routing_rules]]
pattern = { prefix = "/assets/" }
backend = "static"
priority = 90

[[backend.routing_rules]]
pattern = { prefix = "/admin/" }
backend = "fastcgi"
priority = 70
```

### WordPress

```toml
# 公開ページ → 高速、管理画面 → 安定、アップロード → 直接配信
default_backend = "embedded"

[[backend.routing_rules]]
pattern = { prefix = "/wp-admin/" }
backend = "fastcgi"
priority = 80

[[backend.routing_rules]]
pattern = { prefix = "/wp-content/uploads/" }
backend = "static"
priority = 90
```

### ECサイト

```toml
# 商品API → 超高速、商品画像 → 直接配信、決済処理 → 安定性
[[backend.routing_rules]]
pattern = { prefix = "/api/products/" }
backend = "embedded"
priority = 100

[[backend.routing_rules]]
pattern = { prefix = "/images/" }
backend = "static"
priority = 90

[[backend.routing_rules]]
pattern = { prefix = "/checkout/" }
backend = "fastcgi"
priority = 80
```

## アーキテクチャ

```
┌─────────────────────────────────────────────────────────────────┐
│                         fe-php Server                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌───────────────────┐        ┌─────────────────────────────┐   │
│  │  Request Router   │───────▶│   Pattern Matcher           │   │
│  └───────────────────┘        │   (prefix/suffix/regex)     │   │
│          │                    └─────────────────────────────┘   │
│          │                                                       │
│          ├──────────┬───────────────┬─────────────────┐        │
│          │          │               │                 │        │
│          ▼          ▼               ▼                 ▼        │
│  ┌──────────┐ ┌──────────┐  ┌──────────┐     ┌──────────┐    │
│  │ Embedded │ │ FastCGI  │  │  Static  │     │  Admin   │    │
│  │ Backend  │ │ Backend  │  │ Backend  │     │ Console  │    │
│  └──────────┘ └──────────┘  └──────────┘     └──────────┘    │
│       │             │              │                           │
│       ▼             ▼              ▼                           │
│  ┌──────────┐ ┌──────────┐  ┌──────────┐                     │
│  │  libphp  │ │ PHP-FPM  │  │   Disk   │                     │
│  │ (in-proc)│ │ (socket) │  │  (read)  │                     │
│  └──────────┘ └──────────┘  └──────────┘                     │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

## ドキュメント

詳細なドキュメントは`docs/`ディレクトリを参照してください：

- **[HYBRID_BACKEND.md](./HYBRID_BACKEND.md)** - ハイブリッドバックエンドの詳細説明
- **[ARCHITECTURE.md](./ARCHITECTURE.md)** - システム設計と内部構造
- **[FEATURES.md](./FEATURES.md)** - 全機能の詳細説明
- **[USAGE.md](./USAGE.md)** - 設定と運用方法

## パフォーマンスチューニング

### ワーカー数の最適化

```toml
[server]
workers = 16  # CPU core数の1-2倍が推奨
```

### バックエンド選択の指針

| 条件 | 推奨Backend | 理由 |
|------|------------|------|
| レイテンシ < 5ms | Embedded | 最速実行 |
| 実行時間 > 30秒 | FastCGI | ワーカープール保護 |
| メモリ使用量 > 512MB | FastCGI | プロセス分離 |
| リクエスト頻度 > 1000 req/s | Embedded | スループット最大化 |
| レガシーコード | FastCGI | 互換性・安定性 |
| 静的コンテンツ | Static | 最小オーバーヘッド |

### 負荷レベル別推奨構成

#### 低負荷（< 100 RPS）
```toml
[backend]
default_backend = "embedded"  # または "fastcgi"
# どちらでも性能差は小さい
```

#### 中負荷（100-300 RPS）
```toml
[backend]
default_backend = "embedded"  # 1.24x高速
enable_hybrid = true
```

#### 高負荷（300+ RPS）
```toml
[backend]
default_backend = "embedded"  # 1.79x高速（並列50時）
enable_hybrid = true

# 静的ファイルは必ずStaticバックエンドへ
[[backend.routing_rules]]
pattern = { prefix = "/assets/" }
backend = "static"
priority = 100
```

#### 超高負荷（500+ RPS）
```toml
[server]
workers = 16  # CPUコア数に応じて調整

[backend]
default_backend = "embedded"  # 2.14x高速（並列100時）
enable_hybrid = true

# 重い処理はFastCGIで分離
[[backend.routing_rules]]
pattern = { prefix = "/reports/" }
backend = "fastcgi"
priority = 90
```

## 制限事項と今後の計画

### 現在の制限事項

- **FastCGI接続プーリング**: 未実装（毎リクエスト新規接続）
- **Unix Socket**: PHP-FPM Unixソケット接続は未実装（TCP接続のみ）

### 今後の機能

- [ ] **Connection Pooling** - FastCGI接続の再利用
- [ ] **Unix Socket対応** - PHP-FPM Unixソケット接続
- [ ] **HTTP Proxy Backend** - リバースプロキシ機能
- [ ] **Admin Console Phase 2** - ログビューア、メトリクスグラフ、WAF管理、設定変更UI
- [ ] **圧縮対応** - gzip/brotli
- [ ] **Range Request** - HTTP 206 Partial Content対応

## 貢献

プルリクエストを歓迎します。大きな変更を加える場合は、まずissueを開いて変更内容を議論してください。

### 開発環境のセットアップ

```bash
# 依存関係のインストール
cargo build

# テストの実行
cargo test

# フォーマットチェック
cargo fmt --check

# Lintチェック
cargo clippy -- -D warnings

# ベンチマーク実行
fe-php bench --url http://localhost:8080/bench.php --duration 30 --rps 500 --concurrency 50
```

## ライセンス

MIT License - 詳細は[LICENSE](LICENSE)ファイルを参照してください。

---

**ステータス**: アクティブ開発中（Production Ready）
