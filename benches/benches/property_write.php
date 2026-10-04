<?php

declare(strict_types = 1);

$obj = new BenchProps(0, '');

$n = (int) $argv[1];

for ($i = 1; $i <= $n; $i++) {
    $obj->fieldA = $i;
    $obj->fieldB = "value_{$i}";
    $obj->fieldC = ( $i % 2 ) === 0;
    $obj->computed = $i * 2;
}
