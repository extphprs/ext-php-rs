<?php

declare(strict_types = 1);

$a = new BenchProps(42, 'hello');
$b = new BenchProps(42, 'hello');

foreach (range(1, $argv[1]) as $i) {
    $_ = $a <=> $b;
}
