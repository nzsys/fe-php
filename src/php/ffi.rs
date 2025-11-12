use anyhow::{Context, Result};
use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void, c_uint};
use std::path::Path;
use std::ptr;
use std::sync::Mutex;

#[cfg(unix)]
use libloading::os::unix::Library as UnixLibrary;
#[cfg(unix)]
use std::os::raw::c_int as flag_type;

// PHP types (kept for future use)
#[allow(dead_code)]
type ZvalPtr = *mut c_void;

// zend_stream_type enum (PHP 8.1+)
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum ZendStreamType {
    Filename = 0,
    Fp = 1,
    Stream = 2,
}

/// Opaque zend_string type (PHP internal string representation)
#[repr(C)]
pub struct ZendString {
    _private: [u8; 0],
}

/// PHP zend_stream structure
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ZendStream {
    pub handle: *mut c_void,
    pub isatty: c_int,
    pub reader: Option<extern "C" fn(*mut c_void, *mut c_char, usize) -> isize>,
    pub fsizer: Option<extern "C" fn(*mut c_void) -> usize>,
    pub closer: Option<extern "C" fn(*mut c_void)>,
}

/// PHP zend_file_handle structure union
#[repr(C)]
#[derive(Copy, Clone)]
pub union ZendFileHandleUnion {
    pub fp: *mut c_void,  // FILE*
    pub stream: ZendStream,
}

/// PHP zend_file_handle structure (PHP 8.1+)
/// Based on PHP 8.4 zend_stream.h
#[repr(C)]
pub struct ZendFileHandle {
    pub handle: ZendFileHandleUnion,
    pub filename: *mut ZendString,
    pub opened_path: *mut ZendString,
    pub handle_type: u8,  // ZendStreamType
    pub primary_script: bool,
    pub in_list: bool,
    pub buf: *mut c_char,
    pub buf_len: usize,
}

/// PHP SAPI module structure
#[repr(C)]
pub struct SapiModule {
    pub name: *const c_char,
    pub pretty_name: *const c_char,
    pub startup: Option<extern "C" fn(*mut SapiModule) -> c_int>,
    pub shutdown: Option<extern "C" fn(*mut SapiModule) -> c_int>,
    pub activate: Option<extern "C" fn() -> c_int>,
    pub deactivate: Option<extern "C" fn() -> c_int>,
    pub ub_write: Option<extern "C" fn(*const c_char, c_uint) -> c_uint>,
    pub flush: Option<extern "C" fn(*mut c_void)>,
    pub get_stat: Option<extern "C" fn() -> *mut c_void>,
    pub getenv: Option<extern "C" fn(*const c_char, c_uint) -> *mut c_char>,
    pub sapi_error: Option<extern "C" fn(c_int, *const c_char)>,
    pub header_handler: Option<extern "C" fn(*mut c_void, *mut c_void) -> c_int>,
    pub send_headers: Option<extern "C" fn(*mut c_void) -> c_int>,
    pub send_header: Option<extern "C" fn(*mut c_void, *mut c_void)>,
    pub read_post: Option<extern "C" fn(*mut c_char, c_uint) -> c_uint>,
    pub read_cookies: Option<extern "C" fn() -> *mut c_char>,
    pub register_server_variables: Option<extern "C" fn(*mut c_void)>,
    pub log_message: Option<extern "C" fn(*const c_char, c_int)>,
    pub get_request_time: Option<extern "C" fn() -> f64>,
    pub terminate_process: Option<extern "C" fn()>,
    pub php_ini_path_override: *mut c_char,
    pub default_post_reader: Option<extern "C" fn()>,
    pub treat_data: Option<extern "C" fn(c_int, *mut c_char, *mut c_void)>,
    pub executable_location: *mut c_char,
    pub php_ini_ignore: c_int,
    pub php_ini_ignore_cwd: c_int,
    pub get_fd: Option<extern "C" fn(*mut c_int) -> c_int>,
    pub force_http_10: Option<extern "C" fn() -> c_int>,
    pub get_target_uid: Option<extern "C" fn(*mut c_int) -> c_int>,
    pub get_target_gid: Option<extern "C" fn(*mut c_int) -> c_int>,
    pub input_filter: Option<extern "C" fn(c_int, *const c_char, *mut *mut c_char, c_uint, *mut c_uint) -> c_uint>,
    pub ini_defaults: Option<extern "C" fn(*mut c_void)>,
    pub phpinfo_as_text: c_int,
    pub ini_entries: *mut c_char,
    pub additional_functions: *const c_void,
    pub input_filter_init: Option<extern "C" fn() -> c_uint>,
}

// Output buffer storage (thread-local with optimized capacity)
// Most PHP responses are < 64KB, pre-allocate to avoid reallocations
thread_local! {
    static OUTPUT_BUFFER: Mutex<Vec<u8>> = Mutex::new(Vec::with_capacity(65536));
}

/// Callback for PHP output - captures to thread-local buffer
extern "C" fn php_output_handler(output: *const c_char, output_len: c_uint) -> c_uint {
    tracing::debug!("php_output_handler called: ptr={:?}, len={}", output, output_len);

    if output.is_null() || output_len == 0 {
        tracing::debug!("php_output_handler: null or zero length, returning 0");
        return 0;
    }

    unsafe {
        let data = std::slice::from_raw_parts(output as *const u8, output_len as usize);
        tracing::debug!("php_output_handler: captured {} bytes", data.len());
        OUTPUT_BUFFER.with(|buf| {
            if let Ok(mut buffer) = buf.lock() {
                buffer.extend_from_slice(data);
                tracing::debug!("php_output_handler: buffer now has {} bytes total", buffer.len());
            }
        });
    }

    output_len
}

/// SAPI activate callback - called at the start of each request
extern "C" fn php_sapi_activate() -> c_int {
    0 // SUCCESS
}

/// SAPI deactivate callback - called at the end of each request
extern "C" fn php_sapi_deactivate() -> c_int {
    0 // SUCCESS
}

/// Stub callback for registering server variables
/// PHP calls this during request startup to populate $_SERVER
extern "C" fn php_register_variables(_track_vars_array: *mut c_void) {
    // Stub implementation - in production, this would populate $_SERVER with CGI variables
    // For now, we just need this to exist so PHP doesn't crash
}

/// Stub callback for reading POST data
extern "C" fn php_read_post(_buffer: *mut c_char, _count: c_uint) -> c_uint {
    // Stub implementation - return 0 (no POST data)
    0
}

/// Stub callback for reading cookies
extern "C" fn php_read_cookies() -> *mut c_char {
    // Stub implementation - return null (no cookies)
    ptr::null_mut()
}

/// Stub callback for logging messages
extern "C" fn php_log_message(message: *const c_char, _syslog_type: c_int) {
    // Log PHP messages to stderr
    if !message.is_null() {
        unsafe {
            let msg = CStr::from_ptr(message);
            if let Ok(s) = msg.to_str() {
                eprintln!("[PHP] {}", s);
            }
        }
    }
}

/// Stub callback for flushing output
extern "C" fn php_flush(_server_context: *mut c_void) {
    // Stub implementation - we buffer everything until request completes
}

/// Header handler callback
extern "C" fn php_header_handler(
    _sapi_header: *mut c_void,
    _sapi_header_struct: *mut c_void,
) -> c_int {
    0 // SAPI_HEADER_ADD
}

/// Send headers callback
extern "C" fn php_send_headers(_sapi_headers_struct: *mut c_void) -> c_int {
    200 // Return HTTP 200 OK
}

/// PHP FFI bindings
pub struct PhpFfi {
    #[allow(dead_code)]
    library: Library,
    // Function pointers
    sapi_startup: Symbol<'static, unsafe extern "C" fn(*mut SapiModule) -> c_int>,
    php_module_startup: Symbol<'static, unsafe extern "C" fn(*mut SapiModule, *mut c_void) -> c_int>,
    php_module_shutdown: Symbol<'static, unsafe extern "C" fn() -> c_int>,
    php_request_startup: Symbol<'static, unsafe extern "C" fn() -> c_int>,
    php_request_shutdown: Symbol<'static, unsafe extern "C" fn(*mut c_void) -> c_void>,
    php_execute_script: Symbol<'static, unsafe extern "C" fn(*mut ZendFileHandle) -> c_int>,
    zend_stream_init_filename: Symbol<'static, unsafe extern "C" fn(*mut ZendFileHandle, *const c_char)>,
    zend_stream_open: Symbol<'static, unsafe extern "C" fn(*mut ZendFileHandle) -> c_int>,
    zend_destroy_file_handle: Symbol<'static, unsafe extern "C" fn(*mut ZendFileHandle)>,
    sapi_module: *mut SapiModule,
    // Keep CStrings alive for the lifetime of PhpFfi
    _sapi_name: Box<CString>,
    _sapi_pretty_name: Box<CString>,
}

impl PhpFfi {
    /// Load libphp.so and bind functions
    pub fn load<P: AsRef<Path>>(library_path: P) -> Result<Self> {
        let library = unsafe {
            // Use platform-specific loading flags for better compatibility
            #[cfg(unix)]
            {
                // Use RTLD_LAZY | RTLD_LOCAL to prevent symbol conflicts
                // RTLD_NOW was still causing segfaults, trying lazy symbol resolution
                //
                // RTLD_LAZY (1): Resolve symbols on demand (may prevent initialization issues)
                // RTLD_LOCAL (0): Keep symbols local to this library (prevents conflicts)
                const RTLD_LAZY: flag_type = 1;
                const RTLD_LOCAL: flag_type = 0;

                tracing::info!(
                    "Loading libphp from {:?} with RTLD_LAZY | RTLD_LOCAL flags",
                    library_path.as_ref()
                );

                let unix_lib = UnixLibrary::open(
                    Some(library_path.as_ref()),
                    RTLD_LAZY | RTLD_LOCAL
                ).with_context(|| format!("Failed to load libphp from {:?}", library_path.as_ref()))?;

                Library::from(unix_lib)
            }

            #[cfg(not(unix))]
            {
                Library::new(library_path.as_ref())
                    .with_context(|| format!("Failed to load libphp from {:?}", library_path.as_ref()))?
            }
        };

        // Load function symbols
        let sapi_startup = unsafe {
            let symbol: Symbol<unsafe extern "C" fn(*mut SapiModule) -> c_int> =
                library.get(b"sapi_startup\0")
                    .context("Failed to load sapi_startup")?;
            std::mem::transmute(symbol)
        };

        let php_module_startup = unsafe {
            let symbol: Symbol<unsafe extern "C" fn(*mut SapiModule, *mut c_void) -> c_int> =
                library.get(b"php_module_startup\0")
                    .context("Failed to load php_module_startup")?;
            std::mem::transmute(symbol)
        };

        let php_module_shutdown = unsafe {
            let symbol: Symbol<unsafe extern "C" fn() -> c_int> =
                library.get(b"php_module_shutdown\0")
                    .context("Failed to load php_module_shutdown")?;
            std::mem::transmute(symbol)
        };

        let php_request_startup = unsafe {
            let symbol: Symbol<unsafe extern "C" fn() -> c_int> =
                library.get(b"php_request_startup\0")
                    .context("Failed to load php_request_startup")?;
            std::mem::transmute(symbol)
        };

        let php_request_shutdown = unsafe {
            let symbol: Symbol<unsafe extern "C" fn(*mut c_void) -> c_void> =
                library.get(b"php_request_shutdown\0")
                    .context("Failed to load php_request_shutdown")?;
            std::mem::transmute(symbol)
        };

        let php_execute_script = unsafe {
            let symbol: Symbol<unsafe extern "C" fn(*mut ZendFileHandle) -> c_int> =
                library.get(b"php_execute_script\0")
                    .context("Failed to load php_execute_script")?;
            std::mem::transmute(symbol)
        };

        // Load zend_stream_init_filename (PHP 8.1+, required for proper file handle initialization)
        let zend_stream_init_filename = unsafe {
            let symbol: Symbol<unsafe extern "C" fn(*mut ZendFileHandle, *const c_char)> =
                library.get(b"zend_stream_init_filename\0")
                    .context("Failed to load zend_stream_init_filename")?;
            std::mem::transmute(symbol)
        };

        // Load zend_stream_open (PHP 8.1+, opens the file handle)
        let zend_stream_open = unsafe {
            let symbol: Symbol<unsafe extern "C" fn(*mut ZendFileHandle) -> c_int> =
                library.get(b"zend_stream_open\0")
                    .context("Failed to load zend_stream_open")?;
            std::mem::transmute(symbol)
        };

        // Load zend_destroy_file_handle (for proper cleanup)
        let zend_destroy_file_handle = unsafe {
            let symbol: Symbol<unsafe extern "C" fn(*mut ZendFileHandle)> =
                library.get(b"zend_destroy_file_handle\0")
                    .context("Failed to load zend_destroy_file_handle")?;
            std::mem::transmute(symbol)
        };

        // Get SAPI module pointer
        let sapi_module: *mut SapiModule = unsafe {
            let symbol: Symbol<*mut SapiModule> = library.get(b"sapi_module\0")
                .context("Failed to load sapi_module")?;
            *symbol
        };

        // Create CStrings that will live for the lifetime of PhpFfi
        let sapi_name = Box::new(CString::new("fe-php").unwrap());
        let sapi_pretty_name = Box::new(CString::new("fe-php embedded").unwrap());

        Ok(Self {
            library,
            sapi_startup,
            php_module_startup,
            php_module_shutdown,
            php_request_startup,
            php_request_shutdown,
            php_execute_script,
            zend_stream_init_filename,
            zend_stream_open,
            zend_destroy_file_handle,
            sapi_module,
            _sapi_name: sapi_name,
            _sapi_pretty_name: sapi_pretty_name,
        })
    }

    /// Initialize PHP module
    pub fn module_startup(&self) -> Result<()> {
        tracing::info!("=== Starting PHP Module Initialization ===");

        unsafe {
            // Configure SAPI module
            tracing::info!("Step 1: Checking SAPI module pointer...");
            if self.sapi_module.is_null() {
                tracing::error!("SAPI module pointer is NULL!");
                return Err(anyhow::anyhow!(
                    "SAPI module pointer is null - PHP library may not be properly loaded"
                ));
            }
            tracing::info!("SAPI module pointer is valid: {:p}", self.sapi_module);

            tracing::info!("Step 2: Configuring SAPI module...");
            let sapi = &mut *self.sapi_module;

            // Set SAPI name (using the boxed CStrings that live for PhpFfi's lifetime)
            sapi.name = self._sapi_name.as_ptr();
            sapi.pretty_name = self._sapi_pretty_name.as_ptr();
            tracing::info!("SAPI name set: {:?}", CStr::from_ptr(sapi.name));

            // Set required callbacks
            tracing::info!("Step 3: Setting SAPI callbacks...");
            sapi.activate = Some(php_sapi_activate);
            sapi.deactivate = Some(php_sapi_deactivate);
            sapi.ub_write = Some(php_output_handler);
            sapi.flush = Some(php_flush);
            sapi.header_handler = Some(php_header_handler);
            sapi.send_headers = Some(php_send_headers);
            sapi.register_server_variables = Some(php_register_variables);
            sapi.read_post = Some(php_read_post);
            sapi.read_cookies = Some(php_read_cookies);
            sapi.log_message = Some(php_log_message);
            tracing::info!("SAPI callbacks set successfully");

            // Set additional fields to safe defaults
            tracing::info!("Step 4: Setting SAPI defaults...");
            sapi.php_ini_path_override = ptr::null_mut();
            sapi.executable_location = ptr::null_mut();
            sapi.php_ini_ignore = 0;
            sapi.php_ini_ignore_cwd = 0;
            sapi.phpinfo_as_text = 0;
            sapi.ini_entries = ptr::null_mut();
            sapi.additional_functions = ptr::null();
            tracing::info!("SAPI defaults set successfully");

            // Call sapi_startup first (required for embedded SAPI)
            tracing::info!("Step 5: Calling sapi_startup()...");
            tracing::info!("SAPI module address: {:p}", self.sapi_module);

            let result = (self.sapi_startup)(self.sapi_module);

            if result != 0 {
                tracing::error!("sapi_startup() failed with code: {}", result);
                return Err(anyhow::anyhow!(
                    "sapi_startup failed with code {} - SAPI initialization failed",
                    result
                ));
            }

            tracing::info!("sapi_startup() completed successfully with code: {}", result);

            // Call PHP module startup
            tracing::info!("Step 6: Calling php_module_startup()...");

            let result = (self.php_module_startup)(self.sapi_module, ptr::null_mut());

            if result != 0 {
                tracing::error!("php_module_startup() failed with code: {}", result);
                return Err(anyhow::anyhow!(
                    "php_module_startup failed with code {} - check PHP error log for details",
                    result
                ));
            }

            tracing::info!("php_module_startup() completed successfully with code: {}", result);
        }

        tracing::info!("=== PHP Module Initialization Complete ===");
        Ok(())
    }

    /// Shutdown PHP module
    pub fn module_shutdown(&self) -> Result<()> {
        tracing::debug!("Initiating PHP module shutdown...");
        unsafe {
            let result = (self.php_module_shutdown)();
            if result != 0 {
                tracing::error!("php_module_shutdown failed with code {}", result);
                return Err(anyhow::anyhow!(
                    "php_module_shutdown failed with code {} - PHP may not shut down cleanly",
                    result
                ));
            }
        }
        tracing::debug!("PHP module shutdown completed successfully");
        Ok(())
    }

    /// Start a PHP request
    pub fn request_startup(&self) -> Result<()> {
        tracing::info!("=== Starting PHP Request ===");

        // Clear output buffer (preserves capacity for reuse - buffer pooling)
        tracing::debug!("Clearing output buffer...");
        OUTPUT_BUFFER.with(|buf| {
            if let Ok(mut buffer) = buf.lock() {
                buffer.clear(); // Keeps allocated memory for next request

                // Shrink if buffer grew too large (> 1MB) to prevent memory bloat
                if buffer.capacity() > 1024 * 1024 {
                    buffer.shrink_to(65536);
                }
            } else {
                tracing::warn!("Failed to acquire output buffer lock in request_startup");
            }
        });
        tracing::debug!("Output buffer cleared");

        unsafe {
            tracing::info!("Calling php_request_startup()...");
            let result = (self.php_request_startup)();
            if result != 0 {
                tracing::error!("php_request_startup() failed with code: {}", result);
                return Err(anyhow::anyhow!(
                    "php_request_startup failed with code {} - PHP may not be properly initialized",
                    result
                ));
            }
            tracing::info!("php_request_startup() completed successfully");
        }

        tracing::info!("=== PHP Request Started ===");
        Ok(())
    }

    /// Shutdown a PHP request
    pub fn request_shutdown(&self) {
        tracing::info!("=== Shutting down PHP Request ===");
        unsafe {
            tracing::debug!("Calling php_request_shutdown()...");
            (self.php_request_shutdown)(ptr::null_mut());
            tracing::debug!("php_request_shutdown() completed");
        }
        tracing::info!("=== PHP Request Shutdown Complete ===");
    }

    /// Execute a PHP script using embedded libphp
    pub fn execute_script(&self, script_path: &str) -> Result<Vec<u8>> {
        tracing::info!("=== Executing PHP Script: {} ===", script_path);

        // Verify file exists
        tracing::debug!("Step 1: Verifying script exists...");
        let path = Path::new(script_path);
        if !path.exists() {
            tracing::error!("Script not found: {}", script_path);
            return Err(anyhow::anyhow!("PHP script not found: {}", script_path));
        }
        tracing::debug!("Script exists: {}", script_path);

        // Verify file is readable
        tracing::debug!("Step 2: Verifying script is readable...");
        if let Err(e) = std::fs::metadata(path) {
            tracing::error!("Cannot access script {}: {}", script_path, e);
            return Err(anyhow::anyhow!(
                "Cannot access PHP script {}: {}",
                script_path,
                e
            ));
        }
        tracing::debug!("Script is readable");

        unsafe {
            // Create CString for the script path (must live for the duration of the call)
            tracing::debug!("Step 3: Creating CString for script path...");
            let path_cstr = CString::new(script_path)
                .with_context(|| format!("Invalid script path (contains null byte): {}", script_path))?;
            tracing::debug!("CString created successfully");

            // Create zend_file_handle structure (PHP 8.1+)
            tracing::info!("Step 4: Initializing zend_file_handle...");
            let mut file_handle: ZendFileHandle = std::mem::zeroed();
            tracing::info!("  - zend_file_handle zeroed");

            // Step 5: Initialize file handle with zend_stream_init_filename
            tracing::info!("Step 5: Initializing file handle with zend_stream_init_filename...");
            (self.zend_stream_init_filename)(&mut file_handle, path_cstr.as_ptr());
            tracing::info!("  - File handle initialized with filename");

            // Step 6: Open stream with zend_stream_open
            tracing::info!("Step 6: Opening stream with zend_stream_open...");
            let open_result = (self.zend_stream_open)(&mut file_handle);
            if open_result != 0 {
                tracing::error!("zend_stream_open() failed with code: {}", open_result);
                return Err(anyhow::anyhow!("Failed to open PHP script stream: {}", script_path));
            }
            tracing::info!("  - Stream opened successfully");

            // Set primary_script AFTER zend_stream_open (it may reset the flag)
            file_handle.primary_script = true;
            tracing::info!("  - primary_script set to true");

            // Execute the script
            tracing::info!("Step 7: Calling php_execute_script()...");
            tracing::info!("  - About to call php_execute_script with file_handle at {:p}", &file_handle as *const _);
            let result = (self.php_execute_script)(&mut file_handle);
            tracing::info!("  - php_execute_script() returned: {}", result);

            // Clean up file handle (important to avoid memory leaks)
            tracing::debug!("Step 8: Cleaning up file handle...");
            (self.zend_destroy_file_handle)(&mut file_handle);
            tracing::debug!("File handle destroyed");

            // In PHP's embed API: 1 (SUCCESS) = success, 0 (FAILURE) = failure
            if result == 0 {
                // Get output even on error (might contain error messages)
                let output = OUTPUT_BUFFER.with(|buf| {
                    buf.lock().ok().map(|b| b.clone()).unwrap_or_default()
                });

                tracing::error!(
                    "PHP script execution failed with code {} for script: {}",
                    result,
                    script_path
                );

                if !output.is_empty() {
                    // Return output even if script failed (contains error messages)
                    tracing::debug!("Returning error output ({} bytes)", output.len());
                    return Ok(output);
                }

                return Err(anyhow::anyhow!(
                    "PHP script execution failed with code {} for script: {}",
                    result,
                    script_path
                ));
            }
        }

        // Get captured output
        let output = OUTPUT_BUFFER.with(|buf| {
            buf.lock().ok().map(|b| b.clone()).unwrap_or_default()
        });

        tracing::info!("=== PHP Script Execution Complete ({} bytes output) ===", output.len());

        Ok(output)
    }

    /// Get output buffer contents
    pub fn get_output(&self) -> Vec<u8> {
        OUTPUT_BUFFER.with(|buf| {
            buf.lock().ok().map(|b| b.clone()).unwrap_or_default()
        })
    }

    /// Clear output buffer
    pub fn clear_output(&self) {
        OUTPUT_BUFFER.with(|buf| {
            if let Ok(mut buffer) = buf.lock() {
                buffer.clear();
            }
        });
    }
}

// Thread-safe wrapper
unsafe impl Send for PhpFfi {}
unsafe impl Sync for PhpFfi {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // This test requires libphp.so to be installed
    fn test_load_php_library() {
        let result = PhpFfi::load("/usr/local/lib/libphp.so");
        // Will fail if library not found, which is expected in CI
        if result.is_ok() {
            println!("Successfully loaded libphp.so");
        }
    }
}
