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
    pub type_: c_int,  // zend_stream_type (enum = int, not u8!)
    pub primary_script: bool,
    pub in_list: bool,
    pub buf: *mut c_char,
    pub buf_len: usize,
}

/// PHP sapi_headers_struct
#[repr(C)]
pub struct SapiHeadersStruct {
    pub headers: *mut c_void,  // zend_llist
    pub http_response_code: c_int,
    pub http_status_line: *mut c_char,
}

/// PHP sapi_request_info structure (simplified - only fields we need)
#[repr(C)]
pub struct SapiRequestInfo {
    pub request_method: *const c_char,
    pub query_string: *mut c_char,
    pub cookie_data: *mut c_char,
    pub content_length: i64,
    pub path_translated: *mut c_char,
    pub request_uri: *mut c_char,
    // ... other fields exist but we don't need them for now
}

/// PHP sapi_globals_struct (simplified - only fields we need)
#[repr(C)]
pub struct SapiGlobalsStruct {
    pub server_context: *mut c_void,
    pub request_info: SapiRequestInfo,
    pub sapi_headers: SapiHeadersStruct,
    // ... other fields exist but we don't need them for now
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
    tracing::info!("ub_write called! output_len={}, output_ptr={:?}", output_len, output);

    if output.is_null() || output_len == 0 {
        tracing::warn!("ub_write: output is null or length is 0");
        return 0;
    }

    unsafe {
        let data = std::slice::from_raw_parts(output as *const u8, output_len as usize);
        tracing::info!("ub_write: capturing {} bytes: {:?}", data.len(), String::from_utf8_lossy(data));

        OUTPUT_BUFFER.with(|buf| {
            if let Ok(mut buffer) = buf.lock() {
                let old_len = buffer.len();
                buffer.extend_from_slice(data);
                tracing::info!("ub_write: buffer size {} -> {} bytes", old_len, buffer.len());
            } else {
                tracing::error!("ub_write: failed to lock OUTPUT_BUFFER mutex");
            }
        });
    }

    output_len
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

/// SAPI callback for sending headers
/// PHP calls this when it's ready to send HTTP headers
/// For embedded mode, we capture headers via header() calls and handle them separately
/// This callback just signals success to PHP so it continues execution
extern "C" fn php_send_headers(_sapi_headers: *mut c_void) -> c_int {
    // Return SAPI_HEADER_SENT_SUCCESSFULLY (0)
    // In embedded mode, headers are parsed from output buffer after script execution
    0
}

/// PHP FFI bindings
pub struct PhpFfi {
    _library: Library,  // Keep library loaded for the lifetime of PhpFfi
    // Function pointers
    php_embed_init: Symbol<'static, unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int>,
    php_embed_shutdown: Symbol<'static, unsafe extern "C" fn() -> c_void>,
    php_request_startup: Symbol<'static, unsafe extern "C" fn() -> c_int>,
    php_request_shutdown: Symbol<'static, unsafe extern "C" fn(*mut c_void) -> c_void>,
    php_execute_script: Symbol<'static, unsafe extern "C" fn(*mut ZendFileHandle) -> c_int>,
    zend_stream_init_filename: Symbol<'static, unsafe extern "C" fn(*mut ZendFileHandle, *const c_char)>,
    zend_destroy_file_handle: Symbol<'static, unsafe extern "C" fn(*mut ZendFileHandle)>,
    sapi_module: *mut SapiModule,
    // Keep CStrings alive for the lifetime of PhpFfi
    _sapi_name: Box<CString>,
    _sapi_pretty_name: Box<CString>,
    _ini_entries: Box<CString>,
}

impl PhpFfi {
    /// Load libphp.so and bind functions
    pub fn load<P: AsRef<Path>>(library_path: P) -> Result<Self> {
        let library = unsafe {
            // Use platform-specific loading flags for better compatibility
            #[cfg(unix)]
            {
                // For single-worker mode (workers=1), PHP needs RTLD_GLOBAL
                // PHP's internal extensions and opcache require symbols in global namespace
                //
                // RTLD_NOW (2): Resolve all symbols immediately (catch errors early)
                // RTLD_GLOBAL (0x100): Make symbols available globally (required for PHP execution)
                //
                // Note: RTLD_GLOBAL is safe for single-worker mode since there's no
                // symbol conflict between workers. For multi-worker, would need RTLD_LOCAL
                // but that prevents PHP from executing scripts properly.
                const RTLD_NOW: flag_type = 2;
                const RTLD_GLOBAL: flag_type = 0x100;

                tracing::info!(
                    "Loading libphp from {:?} with RTLD_NOW | RTLD_GLOBAL flags",
                    library_path.as_ref()
                );

                let unix_lib = UnixLibrary::open(
                    Some(library_path.as_ref()),
                    RTLD_NOW | RTLD_GLOBAL
                ).with_context(|| format!("Failed to load libphp from {:?}", library_path.as_ref()))?;

                Library::from(unix_lib)
            }

            #[cfg(not(unix))]
            {
                Library::new(library_path.as_ref())
                    .with_context(|| format!("Failed to load libphp from {:?}", library_path.as_ref()))?
            }
        };

        // Load function symbols - use embed SAPI functions
        let php_embed_init = unsafe {
            let symbol: Symbol<unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int> =
                library.get(b"php_embed_init\0")
                    .context("Failed to load php_embed_init")?;
            std::mem::transmute(symbol)
        };

        let php_embed_shutdown = unsafe {
            let symbol: Symbol<unsafe extern "C" fn() -> c_void> =
                library.get(b"php_embed_shutdown\0")
                    .context("Failed to load php_embed_shutdown")?;
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

        // Load zend_destroy_file_handle (for proper cleanup)
        let zend_destroy_file_handle = unsafe {
            let symbol: Symbol<unsafe extern "C" fn(*mut ZendFileHandle)> =
                library.get(b"zend_destroy_file_handle\0")
                    .context("Failed to load zend_destroy_file_handle")?;
            std::mem::transmute(symbol)
        };

        // Get SAPI module pointer (for configuration)
        let sapi_module: *mut SapiModule = unsafe {
            let symbol: Symbol<*mut SapiModule> = library.get(b"sapi_module\0")
                .context("Failed to load sapi_module")?;
            *symbol
        };

        // Create CStrings that will live for the lifetime of PhpFfi
        let sapi_name = Box::new(CString::new("fe-php").unwrap());
        let sapi_pretty_name = Box::new(CString::new("fe-php embedded").unwrap());

        // Create INI entries string to match PHP embed SAPI defaults
        // These settings are critical for proper script execution
        let ini_entries = Box::new(CString::new(
            "html_errors=0\n\
             register_argc_argv=1\n\
             implicit_flush=1\n\
             output_buffering=0\n\
             max_execution_time=0\n\
             max_input_time=-1\n\
             display_errors=1\n\
             display_startup_errors=1\n\
             error_reporting=32767\n\
             log_errors=1\n"
        ).unwrap());

        Ok(Self {
            _library: library,
            php_embed_init,
            php_embed_shutdown,
            php_request_startup,
            php_request_shutdown,
            php_execute_script,
            zend_stream_init_filename,
            zend_destroy_file_handle,
            sapi_module,
            _sapi_name: sapi_name,
            _sapi_pretty_name: sapi_pretty_name,
            _ini_entries: ini_entries,
        })
    }

    /// Initialize PHP embed SAPI
    pub fn module_startup(&self) -> Result<()> {
        unsafe {
            // Configure SAPI module BEFORE php_embed_init
            if self.sapi_module.is_null() {
                return Err(anyhow::anyhow!(
                    "SAPI module pointer is null - PHP library may not be properly loaded"
                ));
            }

            let sapi = &mut *self.sapi_module;

            // Set INI entries before init
            sapi.ini_entries = self._ini_entries.as_ptr() as *mut c_char;

            tracing::info!("Calling php_embed_init()...");

            // php_embed_init(argc, argv) - pass 0, NULL since we don't use CLI args
            let result = (self.php_embed_init)(0, ptr::null_mut());
            if result != 0 {
                return Err(anyhow::anyhow!(
                    "php_embed_init failed with code {} - check PHP installation",
                    result
                ));
            }

            tracing::info!("php_embed_init() completed successfully");

            // CRITICAL: Override callbacks AFTER php_embed_init
            // php_embed_init sets its own ub_write that goes to stdout
            // We need to replace it with our custom handler
            let sapi = &mut *self.sapi_module;

            tracing::info!("Before override - ub_write: {:?}, flush: {:?}", sapi.ub_write, sapi.flush);

            sapi.ub_write = Some(php_output_handler);
            sapi.flush = Some(php_flush);
            sapi.send_headers = Some(php_send_headers);
            sapi.log_message = Some(php_log_message);

            tracing::info!("After override - ub_write: {:?}, flush: {:?}", sapi.ub_write, sapi.flush);
            tracing::info!("SAPI callbacks overridden for output buffering");
        }

        Ok(())
    }

    /// Shutdown PHP embed SAPI
    pub fn module_shutdown(&self) -> Result<()> {
        tracing::info!("Calling php_embed_shutdown()...");
        unsafe {
            (self.php_embed_shutdown)();
        }
        tracing::info!("PHP embed shutdown completed");
        Ok(())
    }

    /// Start a PHP request
    pub fn request_startup(&self) -> Result<()> {
        tracing::info!("request_startup: Starting PHP request...");

        // Clear output buffer (preserves capacity for reuse - buffer pooling)
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

        unsafe {
            // Note: SAPI globals initialization is disabled for now due to complex structure definitions
            // TODO: Properly define zend_llist and other structures to enable this
            // if !self.sapi_globals.is_null() {
            //     (*self.sapi_globals).server_context = 1 as *mut c_void;
            //     (*self.sapi_globals).sapi_headers.http_response_code = 200;
            // }

            tracing::info!("request_startup: Calling php_request_startup()...");
            let result = (self.php_request_startup)();
            tracing::info!("request_startup: php_request_startup() returned: {}", result);

            if result != 0 {
                tracing::error!("php_request_startup failed with code {}", result);
                return Err(anyhow::anyhow!(
                    "php_request_startup failed with code {} - PHP may not be properly initialized",
                    result
                ));
            }

            tracing::info!("request_startup: PHP request started successfully");
        }
        Ok(())
    }

    /// Shutdown a PHP request
    pub fn request_shutdown(&self) {
        tracing::info!("request_shutdown: Shutting down PHP request...");
        unsafe {
            (self.php_request_shutdown)(ptr::null_mut());
        }
        tracing::info!("request_shutdown: PHP request shutdown completed");
    }

    /// Execute a PHP script using embedded libphp
    pub fn execute_script(&self, script_path: &str) -> Result<Vec<u8>> {
        // Verify file exists
        let path = Path::new(script_path);
        if !path.exists() {
            return Err(anyhow::anyhow!("PHP script not found: {}", script_path));
        }

        // Verify file is readable
        if let Err(e) = std::fs::metadata(path) {
            return Err(anyhow::anyhow!(
                "Cannot access PHP script {}: {}",
                script_path,
                e
            ));
        }

        unsafe {
            // Create CString for the script path (must live for the duration of the call)
            let path_cstr = CString::new(script_path)
                .with_context(|| format!("Invalid script path (contains null byte): {}", script_path))?;

            // Create zend_file_handle structure (PHP 8.1+)
            // Initialize with zeros first
            let mut file_handle: ZendFileHandle = std::mem::zeroed();

            // Initialize file handle using zend_stream_init_filename (PHP 8.1+)
            // This function properly sets up all fields including the union
            (self.zend_stream_init_filename)(&mut file_handle, path_cstr.as_ptr());

            // Mark as primary script (critical for execution)
            file_handle.primary_script = true;

            // Execute the script
            tracing::info!("Executing PHP script: {}", script_path);
            tracing::info!("About to call php_execute_script, checking OUTPUT_BUFFER before...");

            // Check buffer before execution
            let buf_before = OUTPUT_BUFFER.with(|buf| {
                buf.lock().ok().map(|b| b.len()).unwrap_or(0)
            });
            tracing::info!("OUTPUT_BUFFER size before execution: {} bytes", buf_before);

            let result = (self.php_execute_script)(&mut file_handle);

            tracing::info!("php_execute_script returned: {}", result);

            // Check buffer immediately after execution
            let buf_after = OUTPUT_BUFFER.with(|buf| {
                buf.lock().ok().map(|b| b.len()).unwrap_or(0)
            });
            tracing::info!("OUTPUT_BUFFER size after execution: {} bytes", buf_after);

            // Get output buffer BEFORE cleanup
            let output = OUTPUT_BUFFER.with(|buf| {
                buf.lock().ok().map(|b| b.clone()).unwrap_or_default()
            });

            tracing::info!(
                "php_execute_script returned: {}, output buffer size: {} bytes",
                result,
                output.len()
            );

            // Log output buffer content for debugging
            if !output.is_empty() {
                let preview = String::from_utf8_lossy(&output[..output.len().min(200)]);
                tracing::info!("Output buffer preview: {:?}", preview);
            } else {
                tracing::warn!("Output buffer is empty!");
            }

            // Clean up file handle (important to avoid memory leaks)
            (self.zend_destroy_file_handle)(&mut file_handle);

            // FrankenPHP ignores return value and checks EG(exit_status) instead
            // For now, we'll return output regardless of return code
            // TODO: Access EG(exit_status) properly
            if result != 0 {
                tracing::warn!(
                    "php_execute_script returned non-zero: {} (may be exit code, not error)",
                    result
                );
            }
        }

        // Get captured output (again in case it was modified)
        let output = OUTPUT_BUFFER.with(|buf| {
            buf.lock().ok().map(|b| b.clone()).unwrap_or_default()
        });

        tracing::info!("Final output size: {} bytes", output.len());

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
