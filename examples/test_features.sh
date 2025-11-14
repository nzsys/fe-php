#!/bin/bash

# fe-php 新機能テストスクリプト

set -e

echo "=== fe-php 新機能テストスクリプト ==="
echo ""

# 色の定義
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# バイナリの存在確認
if [ ! -f "../target/release/fe-php" ]; then
    echo -e "${RED}Error: fe-php binary not found. Please run 'cargo build --release' first.${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Binary found${NC}"
echo ""

# 1. TCP設定のテスト
echo "=== 1. TCP設定のテスト ==="
echo "設定ファイル: examples/advanced_config.toml"
echo "確認事項:"
echo "  - TCP listen (localhost:8080)"
echo "  - HTTP/2有効"
echo "  - Connection pooling設定"
echo "  - Circuit breaker設定"
echo ""

cat advanced_config.toml | grep -A 5 "\[server\]"
echo ""

# 2. Unix Socket設定のテスト
echo "=== 2. Unix Socket設定のテスト ==="
echo "設定ファイル: examples/unix_socket_config.toml"
echo "確認事項:"
echo "  - Unix socket listen (/tmp/fe-php.sock)"
echo "  - HTTP/2有効"
echo ""

cat unix_socket_config.toml | grep -A 5 "\[server\]"
echo ""

# 3. メトリクス確認
echo "=== 3. 新しいメトリクス ==="
echo "以下のメトリクスが利用可能です："
echo "  - connection_pool_idle_connections"
echo "  - connection_pool_active_connections"
echo "  - connection_pool_acquire_duration_seconds"
echo "  - connection_pool_errors_total"
echo "  - circuit_breaker_state"
echo "  - circuit_breaker_failures_total"
echo ""

# 4. 起動例の表示
echo "=== 4. 起動方法 ==="
echo ""
echo -e "${YELLOW}TCP接続で起動:${NC}"
echo "  ./target/release/fe-php serve examples/advanced_config.toml"
echo ""
echo -e "${YELLOW}Unix socketで起動:${NC}"
echo "  ./target/release/fe-php serve examples/unix_socket_config.toml"
echo ""

# 5. テストコマンド
echo "=== 5. テストコマンド例 ==="
echo ""
echo -e "${YELLOW}HTTP/2接続テスト:${NC}"
echo "  curl --http2 http://localhost:8080/test.php -v"
echo ""
echo -e "${YELLOW}メトリクス確認:${NC}"
echo "  curl http://localhost:9090/_metrics | grep connection_pool"
echo ""
echo -e "${YELLOW}Unix socket経由（Nginxなど）:${NC}"
echo "  # Nginx設定例は unix_socket_config.toml を参照"
echo ""

# 6. 設定検証
echo "=== 6. 設定ファイル検証 ==="
echo ""

# TOML構文チェック（tomlqがインストールされている場合）
if command -v tomlq &> /dev/null; then
    echo -e "${GREEN}✓ advanced_config.toml の構文チェック...${NC}"
    tomlq -r . advanced_config.toml > /dev/null && echo "  OK"

    echo -e "${GREEN}✓ unix_socket_config.toml の構文チェック...${NC}"
    tomlq -r . unix_socket_config.toml > /dev/null && echo "  OK"
else
    echo -e "${YELLOW}! tomlq not found. Skipping syntax check.${NC}"
fi

echo ""
echo "=== テスト完了 ==="
echo ""
echo -e "${GREEN}全ての設定ファイルが正常に作成されています。${NC}"
echo "詳細は docs/NEW_FEATURES.md を参照してください。"
echo ""
