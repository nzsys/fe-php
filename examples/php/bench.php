<?php
header('Content-Type: application/json');
$start = microtime(true);

$data = [];
for ($i = 0; $i < 1000; $i++) {
    $data[] = [
        'id' => $i,
        'name' => 'user_' . $i,
        'score' => rand(1, 100)
    ];
}

$text = str_repeat('test', 100);
for ($i = 0; $i < 100; $i++) {
    $text = strtoupper($text);
    $text = strtolower($text);
}

$sum = 0;
for ($i = 0; $i < 10000; $i++) {
    $sum += sqrt($i) * sin($i);
}

$json = json_encode($data);
$decoded = json_decode($json, true);

$elapsed = microtime(true) - $start;

echo json_encode([
    'elapsed_ms' => round($elapsed * 1000, 2),
    'items' => count($decoded),
    'sum' => round($sum, 2),
    'memory_mb' => round(memory_get_peak_usage() / 1024 / 1024, 2)
]);
