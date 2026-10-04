<?php

declare(strict_types = 1);

$obj = new BenchProps(0, '');

foreach (range(1, $argv[1]) as $i) {
    $obj->fieldA += 2;
    $obj->fieldA++;
    $obj->computed += 1;
    if (( $i % 64 ) === 0) {
        $obj->fieldB = '';
    }
    $obj->fieldB .= 'x';
}
