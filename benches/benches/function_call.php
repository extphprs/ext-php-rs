<?php

declare(strict_types=1);

start_instrumentation();

foreach (range(1, $argv[1]) as $i) {
    bench_function($i);
}

stop_instrumentation();
