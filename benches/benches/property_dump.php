<?php

declare(strict_types=1);

$obj = new BenchProps(42, "hello");

foreach (range(1, $argv[1]) as $i) {
    ob_start();
    var_dump($obj);
    ob_end_clean();
}
