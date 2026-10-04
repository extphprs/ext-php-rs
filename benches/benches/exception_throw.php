<?php

declare(strict_types = 1);

$n = (int) $argv[1];

for ($i = 1; $i <= $n; $i++) {
    try {
        bench_throw();
    } catch (Exception $e) {
        $_ = $e;
    }
}
