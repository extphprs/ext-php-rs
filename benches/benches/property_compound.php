<?php

declare(strict_types = 1);

$obj = new BenchProps(0, '');

$n = (int) $argv[1];

for ($i = 1; $i <= $n; $i++) {
    $obj->fieldA += 2;
    $obj->fieldA++;
    $obj->computed += 1;
    if (( $i % 64 ) === 0) {
        $obj->fieldB = '';
    }
    $obj->fieldB .= 'x';
}
