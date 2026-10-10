<?php

declare(strict_types = 1);

$chunk = str_repeat('a', 1024);

$n = (int) $argv[1];

bench_output_passthrough();

for ($i = 1; $i <= $n; $i++) {
    echo $chunk;
    ob_flush();
}

ob_end_flush();
