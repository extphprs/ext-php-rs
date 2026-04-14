<?php

declare(strict_types=1);

$obj = new BenchProps(42, "hello");

foreach (range(1, $argv[1]) as $i) {
    $_ = $obj->fieldA;
    $_ = $obj->fieldB;
    $_ = $obj->fieldC;
    $_ = $obj->computed;
}
