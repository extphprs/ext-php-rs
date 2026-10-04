<?php

declare(strict_types = 1);

$obj = new BenchClass();

$n = (int) $argv[1];

for ($i = 1; $i <= $n; $i++) {
    $obj->method($i);
}
