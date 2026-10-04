<?php

declare(strict_types = 1);

$n = (int) $argv[1];

for ($i = 1; $i <= $n; $i++) {
    bench_function($i);
}
