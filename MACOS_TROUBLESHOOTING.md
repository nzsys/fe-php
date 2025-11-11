# macOS Troubleshooting Guide

## Segmentation Fault Issues

If you experience segmentation faults on macOS, follow these steps:

### 1. Check PHP Thread Safety (ZTS)

PHP must be compiled with **Zend Thread Safety (ZTS)** enabled for embedded SAPI to work in multi-threaded environments.

```bash
# Check if PHP has thread safety enabled
php -i | grep -i "thread"

# Should show:
# Thread Safety => enabled
```

If Thread Safety is **disabled**, you have two options:

#### Option A: Use Single Worker Mode (Recommended for Testing)

Edit `examples/sample_config.toml`:

```toml
[server]
workers = 1  # Single worker avoids threading issues
```

#### Option B: Recompile PHP with ZTS

```bash
# Example for macOS with Homebrew
./configure \
    --enable-embed \
    --enable-zts \
    --prefix=/usr/local/php-embed \
    [other options...]
make
sudo make install
```

### 2. Check PHP Embed SAPI

Verify that PHP was compiled with embed SAPI:

```bash
# Check if libphp exists
ls -la /usr/local/php-embed/lib/libphp.dylib

# Check PHP compile options
php -i | grep -i "configure"
```

Look for `--enable-embed` in the configure command.

### 3. Verify Configuration

Make sure you're using the correct configuration field:

```toml
[server]
workers = 1  # THIS IS USED (authoritative)

[php]
# worker_pool_size = 8  # THIS IS IGNORED (deprecated)
```

### 4. Check Library Path

Ensure the libphp path in config matches your installation:

```toml
[php]
# Common paths:
# macOS Homebrew: /opt/homebrew/lib/libphp.dylib
# Custom build:    /usr/local/php-embed/lib/libphp.dylib
libphp_path = "/usr/local/php-embed/lib/libphp.dylib"
```

### 5. Test Single Worker First

Always test with a single worker before enabling multiple workers:

```bash
# Build
cargo build --release

# Test with single worker
./target/release/fe-php serve --config examples/sample_config.toml
```

Expected output:
```json
{"message":"Configuring 1 PHP worker(s)"}
{"message":"Initializing PHP module for 1 worker(s)..."}
{"message":"PHP module initialized successfully"}
{"message":"Worker 0 starting initialization..."}
{"message":"Worker 0 initialized successfully"}
{"message":"Worker 0 ready to accept requests"}
{"message":"All PHP workers initialized and ready"}
{"message":"Server listening on http://[::1]:8080"}
```

### 6. Common Issues and Solutions

#### Issue: "Waiting for 8 workers" even when workers = 1

**Solution:** The config fix has been applied. Make sure you pulled the latest changes.

#### Issue: Crash at "Server listening on..."

**Possible causes:**
1. PHP not compiled with ZTS
2. Incompatible PHP version
3. Memory corruption in PHP initialization

**Solutions:**
- Use single worker mode
- Fallback to PHP-FPM mode (set `use_fpm = true`)
- Check PHP version (PHP 8.1+ recommended)

#### Issue: "Failed to load libphp"

**Solutions:**
- Verify path: `ls -la $(path_to_libphp)`
- Check permissions: `ls -l $(path_to_libphp)`
- Verify it's a valid dylib: `file $(path_to_libphp)`

### 7. PHP-FPM Fallback

If embedded mode continues to crash, use PHP-FPM:

```toml
[php]
use_fpm = true
fpm_socket = "127.0.0.1:9000"  # or "/var/run/php-fpm.sock"
```

Start PHP-FPM:
```bash
php-fpm -F
```

### 8. Debug Mode

For detailed debugging, set log level to debug or trace:

```toml
[logging]
level = "trace"  # or "debug"
format = "json"
output = "stdout"
```

## Known Limitations on macOS

1. **Multi-worker mode requires ZTS**: Without ZTS, use `workers = 1`
2. **Performance**: Single worker mode has lower throughput than multiple workers
3. **RTLD_LOCAL**: We use RTLD_LOCAL instead of RTLD_GLOBAL to prevent symbol conflicts

## Getting Help

If issues persist:

1. Collect debug information:
   ```bash
   # PHP info
   php -v
   php -i | grep -E "(Thread|Configure|Embed)"

   # Library info
   file /path/to/libphp.dylib
   otool -L /path/to/libphp.dylib

   # Test run with trace logging
   RUST_LOG=trace ./target/release/fe-php serve --config examples/sample_config.toml 2>&1 | tee fe-php-debug.log
   ```

2. Create an issue at: https://github.com/nzsys/fe-php/issues

Include:
- macOS version
- PHP version and build options
- Full error log
- Configuration file
