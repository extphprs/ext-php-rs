<?php

require __DIR__ . '/_panic_utils.php';

$obj = new PanicClone();
expect_rust_panic(fn() => clone $obj, 'clone panicked');
assert($obj instanceof PanicClone);
assert(panic_test_healthy() === 42);
