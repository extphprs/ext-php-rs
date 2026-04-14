<?php

declare(strict_types=1);

$obj = new BenchProps(0, "");

foreach (range(1, $argv[1]) as $i) {
    $obj->fieldA = $i;
    $obj->fieldB = "value_{$i}";
    $obj->fieldC = ($i % 2 === 0);
    $obj->computed = $i * 2;
}
