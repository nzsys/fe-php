# Hybrid Backend System

## 概要

fe-phpは**Hybrid PHPアプリケーションサーバー**です。libphp直接実行、PHP-FPMプロキシ、静的ファイル配信を1つのサーバーで統合し、リクエストパスに応じて最適なバックエンドを選択できます。

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
│  │ Embedded │ │ FastCGI  │  │  Static  │     │ (Future) │    │
│  │ Backend  │ │ Backend  │  │ Backend  │     │  HTTP    │    │
│  └──────────┘ └──────────┘  └──────────┘     │  Proxy   │    │
│       │             │              │          └──────────┘    │
│       ▼             ▼              ▼                           │
│  ┌──────────┐ ┌──────────┐  ┌──────────┐                     │
│  │  libphp  │ │ PHP-FPM  │  │   Disk   │                     │
│  │ (in-proc)│ │ (socket) │  │  (read)  │                     │
│  └──────────┘ └──────────┘  └──────────┘                     │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

## 3つのバックエンド

### 1. Embedded Backend (libphp)

**最速** - 50-100倍高速（PHP-FPM比）

- **実装**: libphpをプロセス内で直接実行（FrankenPHPスタイル）
- **最適な用途**:
  - APIエンドポイント
  - 頻繁にアクセスされるページ
  - リアルタイム処理
  - 軽量なリクエスト

- **利点**:
  - 高速（プロセス間通信なし）
  - メモリ効率が良い
  - OPcache共有

- **制限事項**:
  -  ワーカープール共有
  - メモリリークの影響範囲が広い
  - PHP拡張の互換性問題の可能性

### 2. FastCGI Backend (PHP-FPM)

**安定** - プロセス分離による信頼性

- **実装**: PHP-FPMへのFastCGIプロトコルプロキシ
- **最適な用途**:
  - 管理パネル
  - 長時間実行タスク
  - レガシーコード
  - レポート生成

- **利点**:
  - プロセス分離
  - グレースフルリスタート
  - 安定性
  - PHP-FPM既存設定の再利用

- **トレードオフ**:
  - FastCGIプロトコルオーバーヘッド
  - ソケット通信のレイテンシ

### 3. Static Backend

**超高速** - PHPオーバーヘッドなし

- **実装**: Rustによる直接ファイル配信
- **最適な用途**:
  - 画像 (.jpg, .png, .webp, .svg)
  - CSS/JavaScript
  - フォント (.woff, .woff2, .ttf)
  - 動画

- **機能**:
  - 自動MIMEタイプ判定
  - ETags & Last-Modified
  - Cache-Control ヘッダー
  - パストラバーサル保護
  - インデックスファイル対応

## パフォーマンス特性

| Backend   | Latency    | Throughput | Use Case                  |
|-----------|------------|------------|---------------------------|
| Embedded  | ~1-2ms     | 50,000 req/s | API, 軽量PHP             |
| FastCGI   | ~5-10ms    | 5,000 req/s  | Admin, 長時間処理         |
| Static    | ~0.1-0.5ms | 100,000 req/s | Images, CSS, JS          |

## 設定

### 基本設定

```toml
[backend]
enable_hybrid = true
default_backend = "embedded"

[backend.static_files]
enable = true
root = "/var/www/public"
index_files = ["index.html", "index.htm"]
```

### Hybrid Mode検証ロジック

fe-phpは、ハイブリッドモードで**Embedded（libphp）とFastCGI（PHP-FPM）の共存**をサポートします。

#### フルハイブリッドモード

両方のバックエンドが利用可能：

```toml
[backend]
enable_hybrid = true

[php]
libphp_path = "/usr/lib/libphp.so"  # libphp存在
fpm_socket = "127.0.0.1:9000"       # PHP-FPM設定
```

**検証結果**: エラーなし（両方のバックエンドが利用可能）

#### FastCGI専用モード

libphpが不在の場合：

```toml
[backend]
enable_hybrid = true

[php]
libphp_path = "/usr/lib/libphp.so"  # ファイルが存在しない
fpm_socket = "127.0.0.1:9000"       # PHP-FPM設定
```

**検証結果**:
```
[i] libphp.so not found at: /usr/lib/libphp.so.
    Embedded backend will not be available (FastCGI/Static only mode)
```

#### Embedded専用モード

fpm_socketが未設定の場合：

```toml
[backend]
enable_hybrid = true

[php]
libphp_path = "/usr/lib/libphp.so"  # libphp存在
fpm_socket = ""                     # 未設定
```

**検証結果**:
```
[i] fpm_socket not configured.
    FastCGI backend will not be available (Embedded/Static only mode)
```

#### 非ハイブリッドモード（従来の排他的モード）

`enable_hybrid = false` の場合、`use_fpm` フラグで排他的に選択：

```toml
[backend]
enable_hybrid = false

[php]
use_fpm = false                     # Embedded使用
libphp_path = "/usr/lib/libphp.so"  # 必須
```

または

```toml
[backend]
enable_hybrid = false

[php]
use_fpm = true                      # FastCGI使用
fpm_socket = "127.0.0.1:9000"       # 必須
```

**重要**: 非ハイブリッドモードでは、設定に応じて**どちらか一方のみ**が有効化されます。

### ルーティングルール

ルールは**優先度の高い順**に評価されます。

#### 1. 完全一致 (Exact Match)

```toml
[[backend.routing_rules]]
pattern = { type = "exact", value = "/_health" }
backend = "embedded"
priority = 100
```

#### 2. プレフィックス一致 (Prefix Match)

```toml
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/api/" }
backend = "embedded"
priority = 90
```

#### 3. サフィックス一致 (Suffix Match)

```toml
[[backend.routing_rules]]
pattern = { type = "suffix", value = ".jpg" }
backend = "static"
priority = 80
```

#### 4. 正規表現 (Regex Match)

```toml
[[backend.routing_rules]]
pattern = { type = "regex", value = "^/api/v\\d+/" }
backend = "embedded"
priority = 60
```

## 実用例

### E-commerceサイト

```toml
# 商品API - 超高速
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/api/products/" }
backend = "embedded"
priority = 90

# 商品画像 - キャッシュ効率
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/images/" }
backend = "static"
priority = 85

# 管理画面 - 安定性重視
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/admin/" }
backend = "fastcgi"
priority = 70

# レポート生成 - 長時間処理
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/reports/" }
backend = "fastcgi"
priority = 70
```

### WordPressサイト

```toml
# 公開ページ - 高速表示
[[backend.routing_rules]]
pattern = { type = "suffix", value = ".php" }
backend = "embedded"
priority = 50

# 管理画面 - 安定性
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/wp-admin/" }
backend = "fastcgi"
priority = 70

# アップロードファイル - 効率的配信
[[backend.routing_rules]]
pattern = { type = "prefix", value = "/wp-content/uploads/" }
backend = "static"
priority = 80
```

## ヘルスチェック

ハイブリッドモード有効時、`/_health`エンドポイントは全バックエンドの状態を返します：

```bash
$ curl http://localhost:8080/_health | jq
{
  "status": "healthy",
  "backends": {
    "embedded": {
      "healthy": true,
      "message": "Embedded backend is healthy",
      "latency_ms": 0
    },
    "fastcgi": {
      "healthy": true,
      "message": "FastCGI backend is reachable",
      "latency_ms": 2
    },
    "static": {
      "healthy": true,
      "message": "Static backend is healthy (root: /var/www/public)",
      "latency_ms": null
    }
  }
}
```

## ルーティング戦略のベストプラクティス

### 1. 優先度の設定

```
100: ヘルスチェック、メトリクス（最優先）
90-95: 高速APIエンドポイント
80-85: 静的ファイル
70-75: 管理画面、長時間処理
50-60: デフォルトPHPファイル
```

### 2. パターンの順序

1. **Exact** → 最も具体的
2. **Prefix** → 特定のディレクトリ
3. **Suffix** → ファイル拡張子
4. **Regex** → 複雑なパターン

### 3. バックエンド選択基準

| 条件                         | 推奨Backend  | 理由                     |
|------------------------------|--------------|--------------------------|
| レイテンシ < 5ms             | Embedded     | 最速実行                 |
| 実行時間 > 30秒              | FastCGI      | ワーカープール保護       |
| メモリ使用量 > 512MB         | FastCGI      | プロセス分離             |
| リクエスト頻度 > 1000 req/s  | Embedded     | スループット最大化       |
| レガシーコード               | FastCGI      | 互換性・安定性           |
| 静的コンテンツ               | Static       | 最小オーバーヘッド       |

## トラブルシューティング

### 問題: "Default backend 'xxx' is not registered"

**原因**: デフォルトバックエンドが有効化されていません。

**解決策**:
```toml
[backend]
default_backend = "embedded"  # or "fastcgi" or "static"

# FastCGI を使う場合は fpm_socket を設定
[php]
fpm_socket = "127.0.0.1:9000"

# Static を使う場合は root を設定
[backend.static_files]
enable = true
root = "/var/www/public"
```

### 問題: FastCGI接続エラー

**原因**: PHP-FPMが起動していないか、ソケットアドレスが間違っています。

**解決策**:
```bash
# PHP-FPMの起動確認
ps aux | grep php-fpm

# ソケット確認
netstat -an | grep 9000

# 設定確認
[php]
fpm_socket = "127.0.0.1:9000"  # TCP
# または
fpm_socket = "/var/run/php-fpm.sock"  # Unix socket（未実装）
```

### 問題: 静的ファイルが見つからない

**原因**: ドキュメントルートの設定が間違っています。

**解決策**:
```toml
[backend.static_files]
enable = true
root = "/absolute/path/to/public"  # 絶対パスを使用

# パーミッション確認
# chmod 755 /absolute/path/to/public
```

## パフォーマンスチューニング

### 1. ワーカー数の最適化

```toml
[server]
workers = 16  # CPU core数の1-2倍
```

### 2. ルーティングルールの最適化

- **頻繁にマッチするルールを上位に配置**
- **正規表現は最小限に**（計算コストが高い）
- **デフォルトバックエンドを適切に設定**

### 3. 静的ファイルのキャッシュ

Static Backendは自動的にCache-Controlヘッダーを設定：

- **フォント**: 1年（immutable）
- **画像**: 1日
- **CSS/JS**: 1時間
- **HTML**: no-cache

## 今後の機能

- [ ] **HTTP Proxy Backend** - リバースプロキシ機能
- [ ] **Unix Socket対応** - PHP-FPMへのUnixソケット接続
- [ ] **Connection Pooling** - FastCGI接続の再利用
- [ ] **Hot Reload** - 設定のホットリロード
- [ ] **Metrics per Backend** - バックエンド別のメトリクス
- [ ] **A/B Testing** - バックエンド別のA/Bテスト

Hybrid Backendシステムは、**パフォーマンス**、**安定性**、**柔軟性**のバランスを取り、用途に応じて最適なバックエンドを選択できる画期的なシステムです。既存のPHP-FPM環境と並行運用可能ですので段階的な移行が可能となります。

詳細な設定例は `examples/hybrid_backend_config.toml` を参照してください。
