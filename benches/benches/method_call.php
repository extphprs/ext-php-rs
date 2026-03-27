<?php

declare(strict_types=1);

$obj = new BenchClass();

foreach (range(1, $argv[1]) as $i) {
    $obj->method($i);
}
