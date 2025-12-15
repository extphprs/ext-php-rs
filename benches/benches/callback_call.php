<?php

declare(strict_types=1);

bench_callback_function(fn ($i) => $i * 2, (int) $argv[1]);
