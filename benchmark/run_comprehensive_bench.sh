#!/bin/bash
set -e
cd "$(dirname "$0")/.."

RESULTS_FILE="benchmark/results_$(date +%Y%m%d_%H%M%S).txt"
mkdir -p benchmark

echo "=========================================="
echo "fe-php Comprehensive Benchmark Suite"
echo "=========================================="
echo "" | tee "$RESULTS_FILE"
echo "Start time: $(date)" | tee -a "$RESULTS_FILE"
echo "" | tee -a "$RESULTS_FILE"

pkill -f "fe-php serve" 2>/dev/null || true
pkill -f "nginx" 2>/dev/null || true
sleep 2

if ! lsof -i :9000 &>/dev/null; then
    echo "Starting PHP-FPM..."
    php-fpm -D
    sleep 2
fi

start_server() {
    local mode=$1
    local port=$2
    local config=$3
    local type=$4

    echo "Starting $mode (Port $port)..."

    if [ "$type" = "nginx" ]; then
        nginx -c "$(pwd)/$config" 2>/dev/null
    else
        ./target/release/fe-php serve --config "$config" > /tmp/bench-server.log 2>&1 &
    fi

    sleep 3

    if curl -s "http://localhost:$port/bench.php" | grep -q "elapsed_ms"; then
        echo "  ✓ Server started successfully"
        return 0
    else
        echo "  ✗ Server failed to start"
        return 1
    fi
}

stop_server() {
    local type=$1

    if [ "$type" = "nginx" ]; then
        pkill -f "nginx: master" 2>/dev/null || true
    else
        pkill -f "fe-php serve" 2>/dev/null || true
    fi

    sleep 2
}

run_benchmark() {
    local mode=$1
    local port=$2
    local phase=$3
    local rps=$4
    local concurrency=$5
    local duration=$6

    echo ""
    echo "  Testing: $phase"
    echo "  Settings: RPS=$rps, Concurrency=$concurrency, Duration=${duration}s"

    ./target/release/fe-php bench \
        --url "http://localhost:$port/bench.php" \
        --duration "$duration" \
        --rps "$rps" \
        --concurrency "$concurrency" \
        2>&1 | tee -a "$RESULTS_FILE"

    echo "" | tee -a "$RESULTS_FILE"
}

run_phase() {
    local phase_name=$1
    local rps=$2
    local concurrency=$3
    local duration=$4

    echo "" | tee -a "$RESULTS_FILE"
    echo "=========================================="
    echo "$phase_name: RPS=$rps, Concurrency=$concurrency"
    echo "=========================================="
    echo "" | tee -a "$RESULTS_FILE"

    # Test Nginx + PHP-FPM
    echo "" | tee -a "$RESULTS_FILE"
    echo "----------------------------------------" | tee -a "$RESULTS_FILE"
    echo "Mode: Nginx+PHP-FPM (Port 8081)" | tee -a "$RESULTS_FILE"
    echo "Phase: $phase_name" | tee -a "$RESULTS_FILE"
    echo "----------------------------------------" | tee -a "$RESULTS_FILE"
    if start_server "Nginx+PHP-FPM" 8081 "benchmark/nginx-fpm.conf" "nginx"; then
        run_benchmark "Nginx+PHP-FPM" 8081 "$phase_name" "$rps" "$concurrency" "$duration"
        stop_server "nginx"
    else
        echo "  SKIPPED: Server failed to start" | tee -a "$RESULTS_FILE"
        stop_server "nginx"
    fi
    sleep 3

    # Test fe-php Embedded
    echo "" | tee -a "$RESULTS_FILE"
    echo "----------------------------------------" | tee -a "$RESULTS_FILE"
    echo "Mode: fe-php_Embedded (Port 8082)" | tee -a "$RESULTS_FILE"
    echo "Phase: $phase_name" | tee -a "$RESULTS_FILE"
    echo "----------------------------------------" | tee -a "$RESULTS_FILE"
    if start_server "fe-php_Embedded" 8082 "benchmark/config-embedded.toml" "fephp"; then
        run_benchmark "fe-php_Embedded" 8082 "$phase_name" "$rps" "$concurrency" "$duration"
        stop_server "fephp"
    else
        echo "  SKIPPED: Server failed to start" | tee -a "$RESULTS_FILE"
        stop_server "fephp"
    fi
    sleep 3

    # Test fe-php FastCGI
    echo "" | tee -a "$RESULTS_FILE"
    echo "----------------------------------------" | tee -a "$RESULTS_FILE"
    echo "Mode: fe-php_FastCGI (Port 8083)" | tee -a "$RESULTS_FILE"
    echo "Phase: $phase_name" | tee -a "$RESULTS_FILE"
    echo "----------------------------------------" | tee -a "$RESULTS_FILE"
    if start_server "fe-php_FastCGI" 8083 "benchmark/config-fastcgi-only.toml" "fephp"; then
        run_benchmark "fe-php_FastCGI" 8083 "$phase_name" "$rps" "$concurrency" "$duration"
        stop_server "fephp"
    else
        echo "  SKIPPED: Server failed to start" | tee -a "$RESULTS_FILE"
        stop_server "fephp"
    fi
    sleep 3

    # Test fe-php Hybrid
    echo "" | tee -a "$RESULTS_FILE"
    echo "----------------------------------------" | tee -a "$RESULTS_FILE"
    echo "Mode: fe-php_Hybrid (Port 8084)" | tee -a "$RESULTS_FILE"
    echo "Phase: $phase_name" | tee -a "$RESULTS_FILE"
    echo "----------------------------------------" | tee -a "$RESULTS_FILE"
    if start_server "fe-php_Hybrid" 8084 "benchmark/config-hybrid.toml" "fephp"; then
        run_benchmark "fe-php_Hybrid" 8084 "$phase_name" "$rps" "$concurrency" "$duration"
        stop_server "fephp"
    else
        echo "  SKIPPED: Server failed to start" | tee -a "$RESULTS_FILE"
        stop_server "fephp"
    fi
    sleep 3
}

run_phase "Phase1_LowLoad" 50 5 30
run_phase "Phase2_MediumLoad" 200 20 30
run_phase "Phase3_HighConcurrency" 500 50 30
run_phase "Phase4_StressTest" 1000 100 30
run_phase "Phase5_LatencyOptimized" 10 1 60

pkill -f "fe-php serve" 2>/dev/null || true
pkill -f "nginx: master" 2>/dev/null || true

echo "" | tee -a "$RESULTS_FILE"
echo "=========================================="
echo "Benchmark Complete"
echo "=========================================="
echo "End time: $(date)" | tee -a "$RESULTS_FILE"
echo "Results saved to: $RESULTS_FILE" | tee -a "$RESULTS_FILE"
echo ""
