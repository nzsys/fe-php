<?php
/**
 * fe-php Example PHP Application
 * Simple demonstration of PHP running under fe-php
 */

?>
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>fe-php - PHP Application Platform</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            max-width: 800px;
            margin: 50px auto;
            padding: 20px;
            background: #f5f5f5;
        }
        .container {
            background: white;
            padding: 40px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        h1 {
            color: #333;
            border-bottom: 3px solid #007bff;
            padding-bottom: 10px;
        }
        .info-box {
            background: #f8f9fa;
            border-left: 4px solid #007bff;
            padding: 15px;
            margin: 20px 0;
        }
        .success {
            color: #28a745;
            font-weight: bold;
        }
        table {
            width: 100%;
            border-collapse: collapse;
            margin: 20px 0;
        }
        th, td {
            padding: 12px;
            text-align: left;
            border-bottom: 1px solid #ddd;
        }
        th {
            background-color: #007bff;
            color: white;
        }
        .footer {
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #ddd;
            color: #666;
            font-size: 14px;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>🚀 fe-php Application Platform</h1>

        <div class="info-box">
            <p class="success">✅ PHP is running successfully under fe-php!</p>
        </div>

        <h2>System Information</h2>
        <table>
            <tr>
                <th>Property</th>
                <th>Value</th>
            </tr>
            <tr>
                <td>PHP Version</td>
                <td><?php echo PHP_VERSION; ?></td>
            </tr>
            <tr>
                <td>Server Software</td>
                <td><?php echo $_SERVER['SERVER_SOFTWARE'] ?? 'fe-php'; ?></td>
            </tr>
            <tr>
                <td>Request Method</td>
                <td><?php echo $_SERVER['REQUEST_METHOD'] ?? 'GET'; ?></td>
            </tr>
            <tr>
                <td>Request URI</td>
                <td><?php echo $_SERVER['REQUEST_URI'] ?? '/'; ?></td>
            </tr>
            <tr>
                <td>Current Time</td>
                <td><?php echo date('Y-m-d H:i:s'); ?></td>
            </tr>
            <tr>
                <td>Memory Usage</td>
                <td><?php echo round(memory_get_usage() / 1024 / 1024, 2); ?> MB</td>
            </tr>
        </table>

        <h2>Loaded Extensions</h2>
        <div class="info-box">
            <p><?php echo implode(', ', get_loaded_extensions()); ?></p>
        </div>

        <h2>Features</h2>
        <ul>
            <li>✅ High-performance HTTP server (Tokio + Hyper)</li>
            <li>✅ PHP worker pool with automatic restarts</li>
            <li>✅ OPcache support for optimal performance</li>
            <li>✅ Built-in WAF (Web Application Firewall)</li>
            <li>✅ Prometheus metrics out of the box</li>
            <li>✅ Structured JSON logging</li>
            <li>✅ Configuration management with validation</li>
            <li>✅ Built-in benchmarking tools</li>
        </ul>

        <div class="footer">
            <p>Powered by <strong>fe-php v0.1.0</strong> - All-in-one PHP Application Platform</p>
            <p>Philosophy: 不易流行・無為自然 (Fueki-Ryūkō & Mui-Shizen)</p>
        </div>
    </div>
</body>
</html>
