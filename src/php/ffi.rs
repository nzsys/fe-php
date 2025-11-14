use anyhow::{Context, Result};
use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void, c_uint};
use std::path::Path;
use std::ptr;
use std::sync::Mutex;

#[cfg(unix)]
use libloading::os::unix::Library as UnixLibrary;

// PHP types
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
    if output.is_null() || output_len == 0 {
        return 0;
    }

    unsafe {
        let data = std::slice::from_raw_parts(output as *const u8, output_len as usize);
        OUTPUT_BUFFER.with(|buf| {
            if let Ok(mut buffer) = buf.lock() {
                buffer.extend_from_slice(data);
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

/// Stub callback for sending headers
extern "C" fn php_send_headers(_sapi_headers: *mut c_void) -> c_int {
    // Return SAPI_HEADER_SENT_SUCCESSFULLY
    1
}

/// Stub callback for sending individual header
extern "C" fn php_send_header(_sapi_header: *mut c_void, _server_context: *mut c_void) {
    // Stub implementation
}

/// Stub callback for reading POST data
#[allow(dead_code)]
extern "C" fn php_read_post_data(_sapi_request_info: *mut c_void) {
    // Stub implementation
}

/// PHP FFI bindings
pub struct PhpFfi {
    _library: Library,
    // Function pointers
    sapi_startup: Symbol<'static, unsafe extern "C" fn(*mut SapiModule)>,
    php_module_startup: Symbol<'static, unsafe extern "C" fn(*mut SapiModule, *mut c_void) -> c_int>,
    php_module_shutdown: Symbol<'static, unsafe extern "C" fn() -> c_int>,
    php_request_startup: Symbol<'static, unsafe extern "C" fn() -> c_int>,
    php_request_shutdown: Symbol<'static, unsafe extern "C" fn(*mut c_void) -> c_void>,
    php_execute_script: Symbol<'static, unsafe extern "C" fn(*mut ZendFileHandle) -> c_int>,
    zend_stream_init_filename: Symbol<'static, unsafe extern "C" fn(*mut ZendFileHandle, *const c_char)>,
    zend_destroy_file_handle: Symbol<'static, unsafe extern "C" fn(*mut ZendFileHandle)>,
    // TSRM functions for ZTS (Zend Thread Safety) support
    php_tsrm_startup_ex: Option<Symbol<'static, unsafe extern "C" fn(c_int) -> c_int>>,
    tsrm_shutdown: Option<Symbol<'static, unsafe extern "C" fn()>>,
    ts_resource_ex: Option<Symbol<'static, unsafe extern "C" fn(c_int, *mut c_void) -> *mut c_void>>,
    ts_free_thread: Option<Symbol<'static, unsafe extern "C" fn()>>,
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
                #[cfg(target_os = "freebsd")]
                const FLAGS: c_int = libc::RTLD_NOW | 0x100; // FreeBSD: RTLD_GLOBAL = 0x100

                #[cfg(target_os = "macos")]
                const FLAGS: c_int = libc::RTLD_NOW | 0x8; // macOS: RTLD_GLOBAL = 0x8

                #[cfg(not(any(target_os = "freebsd", target_os = "macos")))]
                const FLAGS: c_int = libc::RTLD_NOW | libc::RTLD_GLOBAL; // Other Unix: use libc constant

                tracing::info!(
                    "Loading libphp from {:?} (flags: {:#x})",
                    library_path.as_ref(),
                    FLAGS
                );

                let unix_lib = UnixLibrary::open(
                    Some(library_path.as_ref()),
                    FLAGS
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

        // Try to load TSRM functions (only present in ZTS builds)
        let php_tsrm_startup_ex = unsafe {
            library.get::<unsafe extern "C" fn(c_int) -> c_int>(b"php_tsrm_startup_ex\0")
                .ok()
                .map(|symbol| std::mem::transmute(symbol))
        };

        let tsrm_shutdown = unsafe {
            library.get::<unsafe extern "C" fn()>(b"tsrm_shutdown\0")
                .ok()
                .map(|symbol| std::mem::transmute(symbol))
        };

        let ts_resource_ex = unsafe {
            library.get::<unsafe extern "C" fn(c_int, *mut c_void) -> *mut c_void>(b"ts_resource_ex\0")
                .ok()
                .map(|symbol| std::mem::transmute(symbol))
        };

        let ts_free_thread = unsafe {
            library.get::<unsafe extern "C" fn()>(b"ts_free_thread\0")
                .ok()
                .map(|symbol| std::mem::transmute(symbol))
        };

        if php_tsrm_startup_ex.is_some() {
            tracing::info!("ZTS (Zend Thread Safety) functions detected - will initialize TSRM");
        } else {
            tracing::info!("NTS (Non-Thread Safe) build detected - TSRM not available");
        }

        // Create CStrings that will live for the lifetime of PhpFfi
        let sapi_name = Box::new(CString::new("fe-php")
            .context("Failed to create SAPI name CString")?);
        let sapi_pretty_name = Box::new(CString::new("fe-php embedded")
            .context("Failed to create SAPI pretty name CString")?);


        Ok(Self {
            _library: library,
            sapi_startup,
            php_module_startup,
            php_module_shutdown,
            php_request_startup,
            php_request_shutdown,
            php_execute_script,
            zend_stream_init_filename,
            zend_destroy_file_handle,
            php_tsrm_startup_ex,
            tsrm_shutdown,
            ts_resource_ex,
            ts_free_thread,
            sapi_module,
            _sapi_name: sapi_name,
            _sapi_pretty_name: sapi_pretty_name,
        })
    }

    /// Initialize PHP module
    pub fn module_startup(&self) -> Result<()> {
        unsafe {
            // Initialize TSRM for ZTS builds BEFORE configuring SAPI
            if let Some(php_tsrm_startup_ex) = &self.php_tsrm_startup_ex {
                tracing::info!("Initializing PHP TSRM (Thread Safe Resource Manager) for ZTS build...");

                // php_tsrm_startup_ex does all the necessary initialization:
                // - Calls tsrm_startup internally
                // - Allocates PHP-specific resource IDs (compiler_globals, executor_globals, etc.)
                // - Sets up thread-local storage for the main thread
                // Parameter: expected_threads (1 for now, more workers can be added later)
                let result = php_tsrm_startup_ex(1);
                if result != 1 {
                    return Err(anyhow::anyhow!(
                        "php_tsrm_startup_ex failed with code {} - ZTS initialization failed",
                        result
                    ));
                }
                tracing::info!("PHP TSRM initialized successfully (includes thread-local storage)");
            }

            // Configure SAPI module
            if self.sapi_module.is_null() {
                return Err(anyhow::anyhow!(
                    "SAPI module pointer is null - PHP library may not be properly loaded"
                ));
            }

            let sapi = &mut *self.sapi_module;

            // Set SAPI name (using the boxed CStrings that live for PhpFfi's lifetime)
            sapi.name = self._sapi_name.as_ptr();
            sapi.pretty_name = self._sapi_pretty_name.as_ptr();

            // Set required callbacks
            sapi.activate = Some(php_sapi_activate);
            sapi.deactivate = Some(php_sapi_deactivate);
            sapi.ub_write = Some(php_output_handler);
            sapi.flush = Some(php_flush);
            sapi.register_server_variables = Some(php_register_variables);
            sapi.read_post = Some(php_read_post);
            sapi.read_cookies = Some(php_read_cookies);
            sapi.log_message = Some(php_log_message);
            sapi.send_headers = Some(php_send_headers);
            sapi.send_header = Some(php_send_header);

            // Set additional fields to safe defaults
            sapi.php_ini_path_override = ptr::null_mut();
            sapi.executable_location = ptr::null_mut();
            sapi.php_ini_ignore = 0;
            sapi.php_ini_ignore_cwd = 0;
            sapi.phpinfo_as_text = 0;
            sapi.ini_entries = ptr::null_mut();
            sapi.additional_functions = ptr::null();

            tracing::debug!("SAPI module configured: name={:?}", CStr::from_ptr(sapi.name));

            // Call sapi_startup() to initialize SAPI infrastructure (hash tables, etc.)
            // This MUST be called before php_module_startup(), following FrankenPHP's pattern
            tracing::debug!("Calling sapi_startup()...");
            (self.sapi_startup)(self.sapi_module);
            tracing::debug!("sapi_startup() completed successfully");

            // Call PHP module startup (this initializes all PHP modules)
            tracing::debug!("Calling php_module_startup()...");
            let result = (self.php_module_startup)(self.sapi_module, ptr::null_mut());
            if result != 0 {
                return Err(anyhow::anyhow!(
                    "php_module_startup failed with code {} - check PHP error log for details",
                    result
                ));
            }

            tracing::debug!("php_module_startup() completed successfully");
        }

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

            // Shutdown TSRM for ZTS builds AFTER PHP module shutdown
            if let Some(tsrm_shutdown) = &self.tsrm_shutdown {
                tracing::info!("Shutting down TSRM (Thread Safe Resource Manager)...");
                tsrm_shutdown();
                tracing::info!("TSRM shutdown completed");
            }
        }
        tracing::debug!("PHP module shutdown completed successfully");
        Ok(())
    }

    /// Initialize TSRM thread-local resources for the current thread (ZTS only)
    /// This MUST be called once by each worker thread before processing requests
    pub fn thread_init(&self) {
        unsafe {
            if let Some(ts_resource_ex) = &self.ts_resource_ex {
                // Call ts_resource_ex(0, NULL) to allocate thread-local storage for this thread
                // First parameter: 0 = core resources
                // Second parameter: NULL = use current thread ID
                tracing::info!("Initializing TSRM thread-local storage for worker thread");
                let result = ts_resource_ex(0, ptr::null_mut());
                if result.is_null() {
                    tracing::error!("TSRM thread initialization failed - ts_resource_ex returned NULL");
                } else {
                    tracing::info!("TSRM thread-local storage initialized successfully");
                }
            } else {
                tracing::warn!("ts_resource_ex function not available - skipping thread initialization (NTS build?)");
            }
        }
    }

    /// Free TSRM thread-local resources for the current thread (ZTS only)
    /// This MUST be called when a worker thread exits
    pub fn thread_cleanup(&self) {
        unsafe {
            if let Some(ts_free_thread) = &self.ts_free_thread {
                tracing::info!("Freeing TSRM thread-local storage for worker thread");
                ts_free_thread();
                tracing::info!("TSRM thread-local storage freed");
            } else {
                tracing::debug!("ts_free_thread function not available - skipping thread cleanup (NTS build?)");
            }
        }
    }

    /// Start a PHP request
    pub fn request_startup(&self) -> Result<()> {
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
            let result = (self.php_request_startup)();
            if result != 0 {
                tracing::error!("php_request_startup failed with code {}", result);
                return Err(anyhow::anyhow!(
                    "php_request_startup failed with code {} - PHP may not be properly initialized",
                    result
                ));
            }
        }
        Ok(())
    }

    /// Shutdown a PHP request
    pub fn request_shutdown(&self) {
        unsafe {
            (self.php_request_shutdown)(ptr::null_mut());
        }
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

            // Execute the script
            tracing::trace!("Executing PHP script: {}", script_path);
            let result = (self.php_execute_script)(&mut file_handle);

            // Clean up file handle (important to avoid memory leaks)
            (self.zend_destroy_file_handle)(&mut file_handle);

            if result != 0 {
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

        tracing::trace!("PHP script executed successfully, output size: {} bytes", output.len());

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

// SAFETY: PhpFfi is thread-safe for the following reasons:
//
// 1. Send: PhpFfi can be safely transferred between threads because:
//    - Library handle is thread-safe (libloading guarantees this)
//    - Function pointers are 'static and immutable
//    - All fields are either thread-safe or immutable after construction
//    - PHP module is initialized once globally before any threading occurs
//
// 2. Sync: PhpFfi can be safely shared between threads because:
//    - All mutation is protected by the OUTPUT_BUFFER thread-local storage
//    - PHP's internal state is managed per-worker using thread-local storage
//    - Each worker has its own request lifecycle (request_startup/shutdown)
//    - No shared mutable state exists between workers
//
// Note: In single-worker mode (workers=1), there is no concurrency.
// In multi-worker mode, each worker operates independently with its own
// PHP request context, avoiding any race conditions.
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
