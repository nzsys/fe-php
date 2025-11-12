// Simple test to verify PHP output capture
use fe_php::php::ffi::PhpFfi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== Testing PHP Output Capture ===\n");

    // Create test PHP file
    let test_file = "/tmp/test_output.php";
    std::fs::write(test_file, "<?php echo '1'; ?>")?;
    println!("Created test file: {}", test_file);

    // Find libphp
    let libphp_path = if std::path::Path::new("/opt/homebrew/lib/libphp.dylib").exists() {
        "/opt/homebrew/lib/libphp.dylib"
    } else if std::path::Path::new("/usr/local/lib/libphp.so").exists() {
        "/usr/local/lib/libphp.so"
    } else if std::path::Path::new("/usr/lib/libphp.so").exists() {
        "/usr/lib/libphp.so"
    } else {
        eprintln!("ERROR: libphp not found!");
        std::process::exit(1);
    };

    println!("Using libphp: {}", libphp_path);

    // Load PHP
    println!("\n=== Loading PHP ===");
    let php = PhpFfi::load(libphp_path)?;
    println!("PHP loaded successfully");

    // Initialize module
    println!("\n=== Initializing PHP Module ===");
    php.module_startup()?;
    println!("PHP module initialized");

    // Start request
    println!("\n=== Starting Request ===");
    php.request_startup()?;
    println!("Request started");

    // Execute script
    println!("\n=== Executing Script ===");
    let output = php.execute_script(test_file)?;

    println!("\n=== RESULT ===");
    println!("Output length: {} bytes", output.len());
    println!("Output content: {:?}", String::from_utf8_lossy(&output));

    if output.len() > 0 {
        println!("\n✓ SUCCESS: Output captured!");
    } else {
        println!("\n✗ FAILURE: No output captured");
    }

    // Shutdown request
    println!("\n=== Shutting Down Request ===");
    php.request_shutdown();
    println!("Request shutdown complete");

    // Shutdown module
    println!("\n=== Shutting Down PHP Module ===");
    php.module_shutdown()?;
    println!("PHP module shutdown complete");

    Ok(())
}
