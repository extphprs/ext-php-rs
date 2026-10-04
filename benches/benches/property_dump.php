<?php

declare(strict_types = 1);

$obj = new BenchProps(42, 'hello');

$n = (int) $argv[1];

for ($i = 1; $i <= $n; $i++) {
    ob_start();
    var_dump($obj);
    ob_end_clean();
}
