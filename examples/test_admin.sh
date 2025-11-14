#!/bin/bash

# Admin Console テストスクリプト

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== fe-php Admin Console テスト ===${NC}"
echo ""

# 設定確認
echo -e "${YELLOW}1. 設定ファイルの確認${NC}"
echo ""
if grep -q "\[admin\]" examples/hybrid_backend_config.toml; then
    echo -e "${GREEN}✓ [admin]セクションが設定されています${NC}"
    echo ""
    grep -A 5 "\[admin\]" examples/hybrid_backend_config.toml
else
    echo -e "${YELLOW}! [admin]セクションが見つかりません${NC}"
    echo "設定を追加してください"
    exit 1
fi

echo ""
echo -e "${YELLOW}2. Admin Consoleの起動方法${NC}"
echo ""
echo "以下のコマンドでサーバーを起動してください："
echo ""
echo "  ./target/release/fe-php serve examples/hybrid_backend_config.toml"
echo ""
echo "起動ログで以下のメッセージを確認："
echo "  ${GREEN}Admin interface available at http://127.0.0.1:9000${NC}"
echo ""

echo -e "${YELLOW}3. Admin Consoleへのアクセス方法${NC}"
echo ""
echo "Webブラウザで以下のURLにアクセス："
echo "  ${BLUE}http://127.0.0.1:9000/${NC}"
echo ""
echo "または、curlでJSON APIにアクセス："
echo "  ${BLUE}curl http://127.0.0.1:9000/api/status${NC}"
echo ""

echo -e "${YELLOW}4. テストコマンド（サーバー起動後に実行）${NC}"
echo ""
echo "# サーバー状態を取得"
echo "curl http://127.0.0.1:9000/api/status | jq ."
echo ""
echo "# Uptimeのみ表示"
echo "curl -s http://127.0.0.1:9000/api/status | jq -r '.server.uptime_seconds'"
echo ""
echo "# リクエスト数を表示"
echo "curl -s http://127.0.0.1:9000/api/status | jq -r '.metrics.total_requests'"
echo ""
echo "# エラー率を表示"
echo "curl -s http://127.0.0.1:9000/api/status | jq -r '.metrics.error_rate'"
echo ""

echo -e "${YELLOW}5. 利用可能なエンドポイント${NC}"
echo ""
echo "Webダッシュボード:"
echo "  /                - メインダッシュボード（サーバー状態、メトリクス、バックエンド）"
echo "  /metrics         - Prometheusメトリクス（実装予定）"
echo "  /logs            - ログビューア（実装予定）"
echo "  /waf             - WAF統計（実装予定）"
echo "  /backends        - バックエンド詳細（実装予定）"
echo "  /system          - システム情報（実装予定）"
echo ""
echo "JSON API:"
echo "  /api/status      - サーバー状態のJSON"
echo ""

echo -e "${YELLOW}6. セキュリティについて${NC}"
echo ""
echo "⚠️  本番環境での注意事項："
echo "  - host = \"127.0.0.1\" に設定（外部アクセスを防止）"
echo "  - リバースプロキシでBasic認証を設定"
echo "  - HTTPS/TLSを有効化"
echo "  - ファイアウォールで保護"
echo ""
echo "詳細は docs/ADMIN_GUIDE.md を参照してください"
echo ""

echo -e "${GREEN}=== 準備完了 ===${NC}"
echo ""
echo "サーバーを起動して、Admin Consoleをお試しください！"
echo ""
