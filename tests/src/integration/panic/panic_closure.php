<?php

require __DIR__ . '/_panic_utils.php';

$closure = panic_test_closure();
expect_rust_panic(fn() => $closure(), 'closure panicked');
assert(panic_test_healthy() === 42);
