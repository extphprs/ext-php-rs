<?php

declare(strict_types=1);

foreach (range(1, $argv[1]) as $i) {
    bench_function($i);
}
