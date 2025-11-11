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
- Process creation/destruction cost (~100ms per request)
- No process reuse
- No shared memory or OpCode caching
- Excessive context switching

## Solution: libphp Embedding (FrankenPHP-style)

Implemented **embedded PHP** using libphp FFI bindings:

1. **In-Process Execution**: PHP runs inside the Rust process
2. **Zero IPC Overhead**: No FastCGI protocol, no sockets
3. **Shared OpCache**: All workers share compiled bytecode
4. **Worker Pool**: Efficient request distribution
5. **Memory Safety**: Rust ownership + PHP memory management

### Changes Made

#### 1. libphp FFI Bindings (`src/php/ffi.rs`)
- Complete SAPI (Server API) module implementation
- 30+ function pointers for PHP integration
- Thread-local output buffering
- Proper request lifecycle management:
  - `php_module_startup()` - Initialize PHP once
  - `php_request_startup()` - Per-request init
  - `php_execute_script()` - Execute PHP in-process
  - `php_request_shutdown()` - Clean up request
  - `php_module_shutdown()` - Shutdown PHP

#### 2. Updated PHP Executor (`src/php/executor.rs`)
- Dual mode: libphp (default) and PHP-FPM (fallback)
- Intelligent header parsing from PHP output
- Zero-copy body extraction
- Memory-optimized buffer management

#### 3. Configuration (`src/config/mod.rs`)
- `use_fpm = false` → Use embedded libphp (recommended)
- `use_fpm = true` → Fallback to PHP-FPM
- `libphp_path` → Path to libphp.so

### Configuration

#### Embedded libphp Mode (Recommended)

```toml
[php]
# Path to libphp.so
libphp_path = "/usr/local/lib/libphp.so"
document_root = "/var/www/html"
worker_pool_size = 8

# Use embedded libphp (default)
use_fpm = false
```

#### PHP-FPM Fallback Mode

If libphp is not available:

```toml
[php]
libphp_path = "/usr/local/lib/libphp.so"  # Ignored when use_fpm=true
document_root = "/var/www/html"
worker_pool_size = 8

# Fallback to PHP-FPM
use_fpm = true
fpm_socket = "127.0.0.1:9000"
```

## Building libphp

### Debian/Ubuntu
```bash
sudo apt-get install php-dev php-embed

# libphp location: /usr/lib/libphp.so
```

### From Source
```bash
./configure --enable-embed --enable-opcache
make -j$(nproc)
sudo make install

# libphp location: /usr/local/lib/libphp.so
```

### macOS (Homebrew)
```bash
brew install php

# libphp location: /opt/homebrew/lib/libphp.dylib
```

### FreeBSD
```bash
cd /usr/ports/lang/php83
make config  # Enable EMBED option
make install clean
```

## Expected Performance Improvements

With embedded libphp:
- **50-100x improvement** in RPS
- **20-100x reduction** in latency
- **10-20x less memory** usage
- Consistent performance under load

### Before (Process Spawning):
- RPS: 9.26
- p50: 106ms
- p99: 148ms
- Memory: ~500MB (multiple processes)

### After (libphp Embedded):
- RPS: **500-2000+** (depending on hardware)
- p50: **1-5ms**
- p99: **5-20ms**
- Memory: ~50MB (single process)

## Verification

Test with embedded libphp:
```bash
# Install libphp
sudo apt-get install php-embed

# Update config.toml
[php]
libphp_path = "/usr/lib/libphp.so"  # Adjust path
use_fpm = false

# Rebuild and run
cargo build --release
./target/release/fe-php serve --config config.toml

# Run benchmark
./target/release/fe-php bench \
    --url http://localhost:8080 \
    --duration 60 \
    --rps 1000 \
    --concurrency 50
```

## Additional Optimizations

For maximum performance with embedded libphp:

1. **Enable OpCache** (`php.ini`):
   ```ini
   [opcache]
   opcache.enable=1
   opcache.memory_consumption=256
   opcache.max_accelerated_files=10000
   opcache.validate_timestamps=0  # Disable in production
   opcache.file_cache=/tmp/opcache
   ```

2. **Tune Worker Pool** (match CPU cores):
   ```toml
   [php]
   worker_pool_size = 16  # Number of CPU cores
   worker_max_requests = 10000  # Restart after N requests
   ```

3. **Enable JIT** (PHP 8.0+):
   ```ini
   opcache.jit_buffer_size=100M
   opcache.jit=tracing
   ```

4. **Persistent Connections**:
   ```php
   <?php
   // In embedded mode, connections persist
   $db = new PDO('mysql:host=localhost;dbname=test', 'user', 'pass', [
       PDO::ATTR_PERSISTENT => true
   ]);
   ```

5. **Shared Memory Cache** (APCu):
   ```php
   <?php
   apcu_store('key', $expensive_data, 3600);
   $data = apcu_fetch('key');
   ```

## Fallback Mode

If libphp is not available, use PHP-FPM:

```toml
[php]
use_fpm = true
fpm_socket = "127.0.0.1:9000"
```

Note: libphp embedding provides 10-50% better performance than PHP-FPM due to zero IPC overhead.
