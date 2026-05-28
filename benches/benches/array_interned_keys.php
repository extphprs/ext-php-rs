<?php

declare(strict_types=1);

$a = bench_array_with_interned_keys((int) $argv[1]);

print_r($a);