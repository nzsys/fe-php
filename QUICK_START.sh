#!/bin/bash

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}==================================${NC}"
echo -e "${BLUE}  fe-php Admin Console 起動確認  ${NC}"
echo -e "${BLUE}==================================${NC}"
echo ""

echo -e "${YELLOW}1. サーバーを起動してください:${NC}"
echo ""
echo "  ./target/release/fe-php serve --config examples/hybrid_backend_config.toml"
echo ""

echo -e "${YELLOW}2. 起動後、以下にアクセスしてください:${NC}"
echo ""
echo -e "  ${GREEN}Admin Console (Web UI):${NC}"
echo "    http://localhost:9000/"
echo ""
echo -e "  ${GREEN}ダッシュボード:${NC}"
echo "    http://localhost:9000/           ✅ バックエンド状態が表示されます"
echo ""
echo -e "  ${GREEN}実装予定ページ:${NC}"
echo "    http://localhost:9000/metrics    📊 プレースホルダー表示"
echo "    http://localhost:9000/logs       📝 プレースホルダー表示"
echo "    http://localhost:9000/waf        🛡️  プレースホルダー表示"
echo "    http://localhost:9000/backends   🔧 プレースホルダー表示"
echo "    http://localhost:9000/system     💻 プレースホルダー表示"
echo ""
echo -e "  ${GREEN}JSON API:${NC}"
echo "    curl http://localhost:9000/api/status | jq ."
echo ""

echo -e "${YELLOW}3. メトリクスとログの確認:${NC}"
echo ""
echo "  Prometheusメトリクス:"
echo "    curl http://localhost:9090/_metrics"
echo ""
echo "  ログフォーマットを変更して起動:"
echo "    FE_PHP_LOG_FORMAT=pretty ./target/release/fe-php serve --config examples/hybrid_backend_config.toml"
echo ""

echo -e "${GREEN}✅ 修正内容:${NC}"
echo "  1. バックエンド状態が表示されるようになりました"
echo "  2. 未実装ページにプレースホルダーが表示されます"
echo "  3. 設定ファイルにログ・メトリクスの詳細コメント追加"
echo ""

echo -e "${BLUE}詳細は以下のドキュメントを参照:${NC}"
echo "  - ADMIN_UPDATES.md     - 今回の更新内容"
echo "  - START_SERVER.md      - サーバー起動ガイド"
echo "  - docs/ADMIN_GUIDE.md  - Admin Console完全ガイド"
echo ""
