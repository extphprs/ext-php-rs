<?php

declare(strict_types = 1);

$packed = pack('P*', ...range(1, 64));

$n = (int) $argv[1];

for ($i = 1; $i <= $n; $i++) {
    bench_binary_slice_sum($packed);
}
