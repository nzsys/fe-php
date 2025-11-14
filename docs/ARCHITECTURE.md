# fe-php アーキテクチャドキュメント

## システム概要

fe-phpは、Rustで実装された高性能PHPアプリケーションサーバーです。libphp、PHP-FPM、静的ファイル配信の3つのバックエンドを単一プロセスで統合し、リクエストごとに最適なバックエンドを選択する世界初のハイブリッドアーキテクチャを採用しています。

## アーキテクチャ図

```
┌─────────────────────────────────────────────────────────────┐
│                     fe-php プロセス                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │         HTTPサーバー (hyper + tokio)                 │  │
│  │  - TLS/SSL対応                                       │  │
│  │  - HTTP/1.1, HTTP/2                                  │  │
│  │  - 非同期I/O                                         │  │
│  └──────────────────┬───────────────────────────────────┘  │
│                     │                                        │
│  ┌──────────────────▼───────────────────────────────────┐  │
│  │      ミドルウェアレイヤー                            │  │
│  │  - WAF (XSS, SQLi検知)                              │  │
│  │  - レート制限                                        │  │
│  │  - CORS                                              │  │
│  │  - IPフィルタリング                                  │  │
│  │  - 圧縮 (gzip/brotli)                               │  │
│  └──────────────────┬───────────────────────────────────┘  │
│                     │                                        │
│  ┌──────────────────▼───────────────────────────────────┐  │
│  │      バックエンドルーター (パターンマッチング)       │  │
│  │  - Exact: /api/users                                 │  │
│  │  - Prefix: /api/*                                    │  │
│  │  - Suffix: *.jpg                                     │  │
│  │  - Regex: ^/uploads/\d+/.*                          │  │
│  └───┬──────────────┬──────────────┬───────────────────┘  │
│      │              │              │                        │
│  ┌───▼────┐    ┌───▼────┐    ┌───▼────┐                  │
│  │Embedded│    │FastCGI │    │ Static │                  │
│  │Backend │    │Backend │    │Backend │                  │
│  └───┬────┘    └───┬────┘    └───┬────┘                  │
│      │              │              │                        │
│  ┌───▼────┐    ┌───▼────┐    ┌───▼────┐                  │
│  │libphp  │    │PHP-FPM │    │ File   │                  │
│  │直接実行│    │プロキシ│    │I/O     │                  │
│  │(TSRM)  │    │(Pool)  │    │(mmap)  │                  │
│  └────────┘    └────────┘    └────────┘                  │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │      Admin Console (axum + tokio)                    │  │
│  │  - 別ポートで動作 (デフォルト: 9002)                │  │
│  │  - Dashboard (HTML)                                  │  │
│  │  - JSON API (/api/status)                           │  │
│  │  - 読み取り専用                                      │  │
│  │  - localhost バインド (127.0.0.1)                   │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## コアコンポーネント

### 1. HTTPサーバーレイヤー

**実装**: `src/server/mod.rs`

- **非同期ランタイム**: Tokio
- **HTTPライブラリ**: Hyper 1.x
- **TLS**: rustls (OpenSSL非依存)
- **接続管理**: グレースフルシャットダウン、アクティブ接続追跡

**特徴**:
- マルチスレッドワーカープール
- Keep-Alive対応
- HTTP/2サーバープッシュ（将来実装予定）
- WebSocket対応（将来実装予定）

### 2. ミドルウェアレイヤー

各ミドルウェアは独立したモジュールとして実装され、チェーン方式で実行されます。

#### WAF (Web Application Firewall)
**実装**: `src/waf/`

```rust
pub enum WafMode {
    Off,      // 無効
    Learn,    // パターン学習モード
    Detect,   // 検知のみ（ブロックしない）
    Block,    // アクティブブロック
}
```

- SQLインジェクション検知
- XSS (Cross-Site Scripting) 検知
- パストラバーサル検知
- 正規表現ベースのルールエンジン

#### レート制限
**実装**: `src/rate_limit/`

- トークンバケットアルゴリズム
- IPアドレスベース
- カスタマイズ可能なレート
- Redis連携（分散環境対応）

#### CORS
**実装**: `src/server/cors.rs`

- プリフライトリクエスト処理
- オリジン検証
- クレデンシャル対応
- カスタムヘッダー/メソッド

#### IPフィルタリング
**実装**: `src/server/ip_filter.rs`

- CIDR表記対応
- ホワイトリスト/ブラックリストモード
- 優先度制御（deny優先）

#### 圧縮
**実装**: `src/server/compression.rs`

- **gzip**: 互換性重視（compression level 6）
- **brotli**: 圧縮率重視（quality 6）
- Content-Type自動判定
- 最小サイズ閾値（1KB）

### 3. バックエンドルーター

**実装**: `src/backend/router.rs`

パターンベースのルーティングエンジン：

```rust
pub enum PathPattern {
    Exact(String),        // 完全一致: "/api/users"
    Prefix(String),       // プレフィックス: "/api/*"
    Suffix(String),       // サフィックス: "*.jpg"
    Regex(regex::Regex),  // 正規表現: "^/uploads/\d+/.*"
}
```

**ルーティングアルゴリズム**:
1. 優先度順にルールをソート（降順）
2. 各ルールを順次評価
3. 最初にマッチしたバックエンドを選択
4. マッチしない場合はデフォルトバックエンド

### 4. バックエンド実装

#### Embeddedバックエンド

**実装**: `src/backend/embedded.rs`

libphpを直接プロセスに組み込み、PHP-FFI経由で実行します。

**アーキテクチャ**:
```
┌─────────────────────────────────┐
│   fe-php メインスレッド         │
│   (Tokio async runtime)         │
└──────────────┬──────────────────┘
               │
               │ PhpRequest送信
               ▼
┌─────────────────────────────────┐
│   WorkerPool                    │
│   (async-channel)               │
└──────────────┬──────────────────┘
               │
         ┌─────┴─────┬─────┬─────┐
         ▼           ▼     ▼     ▼
     ┌──────┐   ┌──────┐      ┌──────┐
     │Worker│   │Worker│ ...  │Worker│
     │Thread│   │Thread│      │Thread│
     └──┬───┘   └──┬───┘      └──┬───┘
        │          │             │
     ┌──▼────────┐ │             │
     │libphp.so  │ │             │
     │TSRM Context│ │             │
     └───────────┘ │             │
```

- スレッドごとに独立したPHPコンテキスト
- グローバル変数の分離
- メモリリーク防止

**利点**:
- プロセス間通信なし
- 直接制御可能

**欠点**:
- クラッシュ時の影響範囲が大きい
- PHPバージョンと密結合

#### FastCGIバックエンド

**実装**: `src/backend/fastcgi.rs`, `src/php/fastcgi.rs`

PHP-FPMへのプロキシとして動作します。

**プロトコル**:
```
Client → fe-php → FastCGI Protocol → PHP-FPM
                 ↓
          Connection Pool
          (TCP/Unix Socket)
```

**接続プール**:
- 最大接続数: 20（設定可能）
- アイドルタイムアウト: 60秒
- 接続ライフタイム: 3600秒
- TCP/Unixソケット両対応

**利点**:
- 安定性（プロセス分離）
- 既存PHP-FPM資産活用
- 独立したPHP設定

**欠点**:
- FastCGIプロトコルオーバーヘッド
- ネットワーク/ソケットI/O

#### Staticバックエンド

**実装**: `src/backend/static_files.rs`

静的ファイル専用の高速配信エンジン。

**最適化**:
- **ETag**: `SHA256(mtime + size)`
- **Last-Modified**: ファイルシステムmtime
- **Cache-Control**: 拡張子別の最適値
- **MIME検出**: 30種類以上のContent-Type
- **Range Request**: 動画/音声ストリーミング対応
- **圧縮**: gzip/brotli自動圧縮

**セキュリティ**:
- パストラバーサル防止（`../`検知）
- シンボリックリンク制限
- ドットファイル非公開

## Admin Console

### 実装

**場所**: `src/admin/server.rs`

**フレームワーク**: Axum (Tokio非同期ランタイム)

**特徴**:
- メインHTTPサーバーとは独立した別プロセス（別ポート）
- Webベースの管理インターフェース
- リアルタイムサーバー監視
- 読み取り専用（設定変更不可）

### アーキテクチャ

```
┌─────────────────────────────────────────────┐
│         Admin Console Server                │
│         (Port 9002, 127.0.0.1)             │
├─────────────────────────────────────────────┤
│                                              │
│  ┌────────────────────────────────────────┐ │
│  │  Axum Router                           │ │
│  ├────────────────────────────────────────┤ │
│  │  GET /          → dashboard (HTML)    │ │
│  │  GET /api/status → api_status (JSON)  │ │
│  └────────────────────────────────────────┘ │
│                     │                        │
│                     ▼                        │
│  ┌────────────────────────────────────────┐ │
│  │  AdminState (Shared State)            │ │
│  │  - start_time                         │ │
│  │  - メトリクス参照 (TODO)              │ │
│  └────────────────────────────────────────┘ │
│                                              │
└─────────────────────────────────────────────┘
```

### データ構造

```rust
pub struct StatusResponse {
    pub server: ServerInfo,      // バージョン、稼働時間、PID
    pub metrics: CurrentMetrics, // RPS、接続数、リクエスト数
    pub backends: Vec<BackendStatus>, // バックエンド別状態
}
```

### Phase 1実装済み機能

1. **Dashboard (/)**: HTML形式の管理画面
2. **JSON API (/api/status)**: プログラマティックアクセス

### Phase 2/3計画中機能

- `/logs` - リアルタイムログビューア
- `/metrics` - Prometheusメトリクスのグラフ表示
- `/waf` - WAFルール管理
- `/backends` - バックエンド詳細情報
- `/system` - システムリソース情報

### セキュリティ設計

- **ローカルバインド**: デフォルトで `127.0.0.1` のみ
- **IPフィルタリング**: `allowed_ips` による制限
- **読み取り専用**: 設定変更機能なし
- **独立ポート**: メインサーバーと分離

---

## メトリクス・監視

### Prometheusメトリクス

**実装**: `src/metrics/collector.rs`

```rust
// HTTPリクエストメトリクス
http_requests_total{method="GET",status="200"}
http_request_duration_seconds{method="GET"}

// バックエンド別メトリクス
backend_requests_total{backend="embedded",status="success"}
backend_request_duration_seconds{backend="embedded"}
backend_errors_total{backend="embedded",error_type="timeout"}

// PHP内部メトリクス
php_workers{status="idle"}
php_memory_bytes{worker_id="0"}
php_requests_handled_total{worker_id="0"}

// OPcacheメトリクス
opcache_hit_rate_percent
opcache_memory_bytes
opcache_cached_scripts

// FastCGI接続プール
fastcgi_pool_connections
fastcgi_pool_max_connections
```

### ログ出力

**構造化ログ (JSON)**:
```json
{
  "timestamp": "2025-11-14T12:00:00Z",
  "level": "INFO",
  "target": "fe_php::server",
  "fields": {
    "message": "Request processed",
    "method": "GET",
    "path": "/api/users",
    "status": 200,
    "duration_ms": 5,
    "backend": "embedded"
  }
}
```

## スレッドモデル

```
┌─────────────────────────────────────────────────────┐
│              Main Thread (Tokio Runtime)            │
│  - HTTP Accept Loop                                 │
│  - Signal Handlers (SIGTERM, SIGINT, SIGUSR1)      │
│  - Metrics Collection                               │
└──────────────┬──────────────────────────────────────┘
               │
    ┌──────────┴──────────┬──────────────┬──────────┐
    │                     │              │          │
┌───▼────┐          ┌────▼───┐     ┌───▼────┐     │
│Worker  │          │Worker  │     │Worker  │     │
│Thread  │          │Thread  │     │Thread  │    ...
│(HTTP)  │          │(HTTP)  │     │(HTTP)  │     │
└────────┘          └────────┘     └────────┘     │
                                                   │
    ┌──────────────────────────────────────────────┘
    │
┌───▼────────────────────────────────────────┐
│      PHP Worker Pool (Blocking Threads)    │
│  - libphp Execution                        │
│  - TSRM Context per thread                 │
│  - Async Channel Communication             │
└────────────────────────────────────────────┘
```

**スレッド配置**:
- **HTTPワーカー**: CPU数と同じ（デフォルト）
- **PHPワーカー**: 設定可能（デフォルト10）
- 合計スレッド数 = HTTPワーカー + PHPワーカー + メインスレッド

## メモリ管理

### ゼロコピー最適化

- **Bytes**: FastCGIプロトコルでゼロコピーバッファ
- **SmallVec**: ヘッダー用スタックアロケーション
- **Arc**: 設定の共有（コピー不要）

### メモリ制限

```toml
[php]
memory_limit = "128M"        # PHP実行時メモリ制限
max_execution_time = 30      # 最大実行時間

[server]
max_body_size = 10485760     # 最大リクエストボディサイズ (10MB)
```

## エラーハンドリング

### パニック回復

```rust
// libphp実行時のパニック捕捉
std::panic::catch_unwind(|| {
    php_executor.execute(script)
})
```

### グレースフルデグラデーション

1. Embeddedバックエンドクラッシュ → FastCGIフォールバック
2. FastCGI接続失敗 → 接続リトライ（最大3回）
3. 全バックエンド失敗 → 503 Service Unavailable

## セキュリティアーキテクチャ

### 多層防御

```
Internet
    │
    ▼
┌────────────────┐
│  TLS Termination│  ← Let's Encrypt証明書
└────────┬───────┘
         ▼
┌────────────────┐
│  IP Filter     │  ← CIDR制限
└────────┬───────┘
         ▼
┌────────────────┐
│  Rate Limiter  │  ← DDoS対策
└────────┬───────┘
         ▼
┌────────────────┐
│  WAF           │  ← SQLi/XSS検知
└────────┬───────┘
         ▼
┌────────────────┐
│  Backend       │  ← プロセス分離
└────────────────┘
```

## パフォーマンス最適化

### 1. 接続プーリング
- FastCGI接続の再利用
- Keep-Alive有効化
- 接続ウォームアップ

### 2. キャッシング
- OPcacheフル活用（JIT対応）
- 静的ファイルETagキャッシング
- 設定ホットリロード（再起動不要）

### 3. 非同期I/O
- Tokio非同期ランタイム
- epoll/kqueue活用
- ゼロコピーI/O

### 4. CPU最適化
- LTO (Link Time Optimization)
- SIMD命令活用（brotli圧縮）
- アリーナアロケーション

## 将来の拡張

- [ ] HTTP/3 (QUIC)対応
- [ ] WebSocket対応
- [ ] GraphQLネイティブサポート
- [ ] Redis/Memcached統合キャッシュ
- [ ] マルチテナント対応
- [ ] Kubernetes Operator
- [ ] Service Mesh統合

## 参考資料

- [RFC 3875 - CGI仕様](https://tools.ietf.org/html/rfc3875)
- [FastCGI仕様](https://fastcgi-archives.github.io/FastCGI_Specification.html)
- [PHP-FFI](https://www.php.net/manual/en/book.ffi.php)
- [Tokio Documentation](https://tokio.rs/)
- [Hyper HTTP Library](https://hyper.rs/)
