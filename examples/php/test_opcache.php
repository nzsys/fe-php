<?php
/**
 * OPcache Status Test
 * Shows OPcache configuration and statistics
 */

header('Content-Type: application/json');

if (function_exists('opcache_get_status')) {
    $status = opcache_get_status();
    $config = opcache_get_configuration();

    echo json_encode([
        'opcache_enabled' => true,
        'status' => $status,
        'configuration' => $config,
        'memory_usage' => [
            'used' => round($status['memory_usage']['used_memory'] / 1024 / 1024, 2) . ' MB',
            'free' => round($status['memory_usage']['free_memory'] / 1024 / 1024, 2) . ' MB',
            'wasted' => round($status['memory_usage']['wasted_memory'] / 1024 / 1024, 2) . ' MB',
        ],
        'hit_rate' => round($status['opcache_statistics']['opcache_hit_rate'], 2) . '%',
        'cached_scripts' => $status['opcache_statistics']['num_cached_scripts'],
    ], JSON_PRETTY_PRINT);
} else {
    echo json_encode([
        'opcache_enabled' => false,
        'message' => 'OPcache is not enabled'
    ], JSON_PRETTY_PRINT);
}
