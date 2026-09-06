<?php

declare(strict_types = 1);

$packed = pack('P*', ...range(1, 64));

foreach (range(1, $argv[1]) as $i) {
    bench_binary_slice_sum($packed);
}
