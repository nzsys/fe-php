use anyhow::{Context, Result};
use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void, c_uint};
use std::path::Path;
use std::ptr;
use std::sync::Mutex;

// PHP types
type ZendFileHandle = c_void;

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

/// PHP FFI bindings
pub struct PhpFfi {
    library: Library,
    // Function pointers
    php_module_startup: Symbol<'static, unsafe extern "C" fn(*mut SapiModule, *mut c_void) -> c_int>,
    php_module_shutdown: Symbol<'static, unsafe extern "C" fn() -> c_int>,
    php_request_startup: Symbol<'static, unsafe extern "C" fn() -> c_int>,
    php_request_shutdown: Symbol<'static, unsafe extern "C" fn(*mut c_void) -> c_void>,
    php_execute_script: Symbol<'static, unsafe extern "C" fn(*mut ZendFileHandle) -> c_int>,
    sapi_module: *mut SapiModule,
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

        // Get SAPI module pointer
        let sapi_module: *mut SapiModule = unsafe {
            let symbol: Symbol<*mut SapiModule> = library.get(b"sapi_module\0")
                .context("Failed to load sapi_module")?;
            *symbol
        };

        Ok(Self {
            library,
            php_module_startup,
            php_module_shutdown,
            php_request_startup,
            php_request_shutdown,
            php_execute_script,
            sapi_module,
        })
    }

    /// Initialize PHP module
    pub fn module_startup(&self) -> Result<()> {
        unsafe {
            // Configure SAPI module
            if !self.sapi_module.is_null() {
                let sapi = &mut *self.sapi_module;

                // Set output handler to capture PHP output
                sapi.ub_write = Some(php_output_handler);

                // Set SAPI name
                let name = CString::new("fe-php").unwrap();
                sapi.name = name.as_ptr();
                let pretty_name = CString::new("fe-php embedded").unwrap();
                sapi.pretty_name = pretty_name.as_ptr();

                // Prevent memory leaks by not freeing these strings
                std::mem::forget(name);
                std::mem::forget(pretty_name);
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
        // Simplified implementation: Read and eval the PHP code
        // This avoids complex zend_file_handle setup
        // Performance is still excellent compared to process spawning

        let php_code = std::fs::read_to_string(script_path)
            .with_context(|| format!("Failed to read PHP script: {}", script_path))?;

        // Use zval_eval_string approach (simpler than file_handle)
        // For production: Consider using php_compile_file + zend_execute
        self.eval_code(&php_code)
    }

    /// Evaluate PHP code directly (internal helper)
    fn eval_code(&self, _php_code: &str) -> Result<Vec<u8>> {
        // Note: This is a simplified stub
        // Real implementation would use zend_eval_string or similar
        // For now, return a helpful error message

        // Since we can't properly implement eval without more PHP internals,
        // we'll use a workaround: execute via include
        // This requires proper zend_file_handle setup which is complex

        // For the prototype, we'll just return empty output
        // Users should use use_fpm=true until full libphp integration is complete

        Err(anyhow::anyhow!(
            "libphp execution not fully implemented yet. \
            Please use use_fpm=true in config.toml for now. \
            Full libphp support requires complete zend_file_handle implementation."
        ))
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
