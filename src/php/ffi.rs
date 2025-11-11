use anyhow::{Context, Result};
use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void, c_uint};
use std::path::Path;
use std::ptr;
use std::sync::Mutex;

// PHP types
type ZvalPtr = *mut c_void;

// zend_file_handle type constants (PHP 8.1+)
const ZEND_HANDLE_FILENAME: c_int = 0;
const ZEND_HANDLE_FP: c_int = 1;
const ZEND_HANDLE_STREAM: c_int = 2;

/// PHP zend_file_handle structure (PHP 8.1+)
/// This structure changed significantly in PHP 8.1
#[repr(C)]
pub struct ZendFileHandle {
    pub filename: *const c_char,
    pub opened_path: *mut c_char,
    pub handle_type: u8,
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

/// SAPI activate callback
extern "C" fn php_sapi_activate() -> c_int {
    // Called at the start of each request
    0 // SUCCESS
}

/// SAPI deactivate callback
extern "C" fn php_sapi_deactivate() -> c_int {
    // Called at the end of each request
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

/// PHP FFI bindings
pub struct PhpFfi {
    library: Library,
    // Function pointers
    php_module_startup: Symbol<'static, unsafe extern "C" fn(*mut SapiModule, *mut c_void) -> c_int>,
    php_module_shutdown: Symbol<'static, unsafe extern "C" fn() -> c_int>,
    php_request_startup: Symbol<'static, unsafe extern "C" fn() -> c_int>,
    php_request_shutdown: Symbol<'static, unsafe extern "C" fn(*mut c_void) -> c_void>,
    php_execute_script: Symbol<'static, unsafe extern "C" fn(*mut ZendFileHandle) -> c_int>,
    zend_stream_init_filename: Option<Symbol<'static, unsafe extern "C" fn(*mut ZendFileHandle, *const c_char) -> c_int>>,
    sapi_module: *mut SapiModule,
    // Keep CStrings alive for the lifetime of PhpFfi
    _sapi_name: Box<CString>,
    _sapi_pretty_name: Box<CString>,
}

impl PhpFfi {
    /// Load libphp.so and bind functions
    pub fn load<P: AsRef<Path>>(library_path: P) -> Result<Self> {
        let library = unsafe {
            Library::new(library_path.as_ref())
                .with_context(|| format!("Failed to load libphp from {:?}", library_path.as_ref()))?
        };

        // Load function symbols
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

        // Load zend_stream_init_filename (PHP 8.1+ only, optional for backward compatibility)
        let zend_stream_init_filename = unsafe {
            library.get(b"zend_stream_init_filename\0")
                .ok()
                .map(|symbol: Symbol<unsafe extern "C" fn(*mut ZendFileHandle, *const c_char) -> c_int>| {
                    std::mem::transmute(symbol)
                })
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
            php_module_startup,
            php_module_shutdown,
            php_request_startup,
            php_request_shutdown,
            php_execute_script,
            zend_stream_init_filename,
            sapi_module,
            _sapi_name: sapi_name,
            _sapi_pretty_name: sapi_pretty_name,
        })
    }

    /// Initialize PHP module
    pub fn module_startup(&self) -> Result<()> {
        unsafe {
            // Configure SAPI module
            if !self.sapi_module.is_null() {
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

                // Set additional fields to safe defaults
                sapi.php_ini_path_override = ptr::null_mut();
                sapi.executable_location = ptr::null_mut();
                sapi.php_ini_ignore = 0;
                sapi.php_ini_ignore_cwd = 0;
                sapi.phpinfo_as_text = 0;
                sapi.ini_entries = ptr::null_mut();
                sapi.additional_functions = ptr::null();
            }

            // Call PHP module startup
            let result = (self.php_module_startup)(self.sapi_module, ptr::null_mut());
            if result != 0 {
                return Err(anyhow::anyhow!("php_module_startup failed with code {}", result));
            }
        }

        Ok(())
    }

    /// Shutdown PHP module
    pub fn module_shutdown(&self) -> Result<()> {
        unsafe {
            let result = (self.php_module_shutdown)();
            if result != 0 {
                return Err(anyhow::anyhow!("php_module_shutdown failed with code {}", result));
            }
        }
        Ok(())
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
            }
        });

        unsafe {
            let result = (self.php_request_startup)();
            if result != 0 {
                return Err(anyhow::anyhow!("php_request_startup failed with code {}", result));
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
        if !Path::new(script_path).exists() {
            return Err(anyhow::anyhow!("PHP script not found: {}", script_path));
        }

        unsafe {
            // Create CString for the script path (must live for the duration of the call)
            let path_cstr = CString::new(script_path)
                .with_context(|| format!("Invalid script path: {}", script_path))?;

            // Create zend_file_handle structure (PHP 8.1+)
            let mut file_handle = ZendFileHandle {
                filename: path_cstr.as_ptr(),
                opened_path: ptr::null_mut(),
                handle_type: ZEND_HANDLE_FILENAME as u8,
                primary_script: true,
                in_list: false,
                buf: ptr::null_mut(),
                buf_len: 0,
            };

            // Initialize file handle using zend_stream_init_filename if available (PHP 8.1+)
            if let Some(ref init_fn) = self.zend_stream_init_filename {
                let init_result = (init_fn)(&mut file_handle, path_cstr.as_ptr());
                if init_result != 0 {
                    return Err(anyhow::anyhow!(
                        "zend_stream_init_filename failed with code {}: {}",
                        init_result,
                        script_path
                    ));
                }
            }

            // Execute the script
            let result = (self.php_execute_script)(&mut file_handle);

            if result != 0 {
                // Get output even on error (might contain error messages)
                let output = OUTPUT_BUFFER.with(|buf| {
                    buf.lock().ok().map(|b| b.clone()).unwrap_or_default()
                });

                if !output.is_empty() {
                    // Return output even if script failed (contains error messages)
                    return Ok(output);
                }

                return Err(anyhow::anyhow!(
                    "PHP script execution failed with code {}: {}",
                    result,
                    script_path
                ));
            }
        }

        // Get captured output
        let output = OUTPUT_BUFFER.with(|buf| {
            buf.lock().ok().map(|b| b.clone()).unwrap_or_default()
        });

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
