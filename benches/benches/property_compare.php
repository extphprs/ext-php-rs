<?php

declare(strict_types = 1);

$a = new BenchProps(42, 'hello');
$b = new BenchProps(42, 'hello');

$n = (int) $argv[1];

for ($i = 1; $i <= $n; $i++) {
    $_ = $a <=> $b;
}
