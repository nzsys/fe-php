# Performance Analysis and Improvements

## Problem Identified

Benchmark results showed extremely poor performance:
- Target RPS: 100
- Actual RPS: **9.26** (only 9% of target)
- Response time: p50 = 106ms, p99 = 148ms

## Root Cause

The original implementation in `src/php/ffi.rs` was spawning a new PHP process for **every request**:

```rust
let output = Command::new("php")
    .arg(script_path)
    .output()  // ← Spawns new process every time!
```

This causes massive overhead:
- Process creation/destruction cost
- No process reuse
- No connection pooling
- No OpCode caching benefits

## Solution: PHP-FPM Integration

Implemented FastCGI client to communicate with PHP-FPM, which provides:

1. **Process Pooling**: Pre-forked PHP workers
2. **Persistent Connections**: Reuse connections via FastCGI protocol
3. **OpCode Caching**: Shared memory for compiled scripts
4. **Production-Ready**: Battle-tested in millions of deployments

### Changes Made

#### 1. FastCGI Client (`src/php/fastcgi.rs`)
- Complete FastCGI protocol implementation
- Supports TCP sockets and Unix domain sockets
- Async/await compatible
- Handles PARAMS, STDIN, STDOUT, STDERR records

#### 2. Updated PHP Executor (`src/php/executor.rs`)
- Dual mode support: CLI (legacy) and FPM (production)
- Parses FastCGI responses (headers + body)
- Maintains backward compatibility

#### 3. Configuration (`src/config/mod.rs`)
- Added `use_fpm` flag
- Added `fpm_socket` setting (TCP or Unix socket)

### Configuration

Enable PHP-FPM in your `config.toml`:

```toml
[php]
use_fpm = true
fpm_socket = "127.0.0.1:9000"  # or "/var/run/php-fpm.sock"
```

## Setting Up PHP-FPM

### Debian/Ubuntu
```bash
sudo apt-get install php-fpm
sudo systemctl start php-fpm
sudo systemctl enable php-fpm
```

### RedHat/CentOS
```bash
sudo yum install php-fpm
sudo systemctl start php-fpm
sudo systemctl enable php-fpm
```

### macOS (Homebrew)
```bash
brew install php
brew services start php
```

### FreeBSD
```bash
sudo pkg install php-fpm
sudo service php-fpm start
```

### Docker
```dockerfile
FROM php:8.3-fpm
# Your application setup
```

## Expected Performance Improvements

With PHP-FPM, expect:
- **10-100x improvement** in RPS
- **10x reduction** in latency
- Better resource utilization
- Consistent performance under load

### Before (CLI mode):
- RPS: 9.26
- p50: 106ms
- p99: 148ms

### After (PHP-FPM - estimated):
- RPS: 500-1000+ (depending on hardware)
- p50: 5-15ms
- p99: 20-50ms

## Verification

Test with PHP-FPM enabled:
```bash
# Start PHP-FPM (if not running)
sudo systemctl start php-fpm

# Update config.toml
use_fpm = true
fpm_socket = "127.0.0.1:9000"

# Rebuild and run
cargo build --release
./target/release/fe-php serve --config config.toml

# Run benchmark
./target/release/fe-php bench --url http://localhost:8080 --duration 60 --rps 100 --concurrency 10
```

## Additional Optimizations

For maximum performance:

1. **Increase PHP-FPM workers** (`/etc/php-fpm.d/www.conf`):
   ```ini
   pm = dynamic
   pm.max_children = 50
   pm.start_servers = 20
   pm.min_spare_servers = 10
   pm.max_spare_servers = 30
   ```

2. **Enable OpCache** (`php.ini`):
   ```ini
   opcache.enable=1
   opcache.memory_consumption=256
   opcache.max_accelerated_files=10000
   opcache.validate_timestamps=0
   ```

3. **Use Unix sockets** (faster than TCP):
   ```toml
   fpm_socket = "/var/run/php-fpm.sock"
   ```

4. **Tune worker pool**:
   ```toml
   [php]
   worker_pool_size = 16  # Match CPU cores
   ```

## Fallback Mode

If PHP-FPM is not available, set `use_fpm = false` to use CLI mode.
Note: CLI mode has significantly lower performance and is not recommended for production.
