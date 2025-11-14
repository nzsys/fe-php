#!/bin/bash

set -e
cd "$(dirname "$0")/.."

pkill -f fe-php 2>/dev/null || true
pkill -f nginx 2>/dev/null || true
sleep 1

echo "=========================================="
echo "Testing All Modes - Quick Verification"
echo "=========================================="
echo ""

echo "[1/4] fe-php Embedded Mode (Port 8082)"
./target/release/fe-php serve --config benchmark/config-embedded.toml > /tmp/bench-embedded.log 2>&1 &
PID_EMBEDDED=$!
sleep 3
RESULT_EMBEDDED=$(curl -s http://localhost:8082/bench.php)
echo "  Result: $RESULT_EMBEDDED"
kill $PID_EMBEDDED 2>/dev/null || true
sleep 1

echo "[2/4] fe-php FastCGI Mode (Port 8083)"
./target/release/fe-php serve --config benchmark/config-fastcgi-only.toml > /tmp/bench-fastcgi.log 2>&1 &
PID_FASTCGI=$!
sleep 3
RESULT_FASTCGI=$(curl -s http://localhost:8083/bench.php)
echo "  Result: $RESULT_FASTCGI"
kill $PID_FASTCGI 2>/dev/null || true
sleep 1

echo "[3/4] fe-php Hybrid Mode (Port 8084)"
./target/release/fe-php serve --config benchmark/config-hybrid.toml > /tmp/bench-hybrid.log 2>&1 &
PID_HYBRID=$!
sleep 3
RESULT_HYBRID=$(curl -s http://localhost:8084/bench.php)
echo "  Result: $RESULT_HYBRID"
kill $PID_HYBRID 2>/dev/null || true
sleep 1

echo "[4/4] Nginx + PHP-FPM (Port 8081)"
echo "  Skipping (requires root/permissions)"

echo ""
echo "=========================================="
echo "Quick Test Complete"
echo "=========================================="
echo ""
echo "Summary:"
echo "  Embedded: $(echo $RESULT_EMBEDDED | jq -r '.elapsed_ms // "N/A"') ms"
echo "  FastCGI:  $(echo $RESULT_FASTCGI | jq -r '.elapsed_ms // "N/A"') ms"
echo "  Hybrid:   $(echo $RESULT_HYBRID | jq -r '.elapsed_ms // "N/A"') ms"
echo ""
