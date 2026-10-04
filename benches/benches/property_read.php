<?php

declare(strict_types = 1);

$obj = new BenchProps(42, 'hello');

$n = (int) $argv[1];

for ($i = 1; $i <= $n; $i++) {
    $_ = $obj->fieldA;
    $_ = $obj->fieldB;
    $_ = $obj->fieldC;
    $_ = $obj->computed;
}
