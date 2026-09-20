<?php

require __DIR__ . '/_panic_utils.php';

expect_rust_panic(fn() => panic_test_function(), 'function panicked');
expect_rust_panic(fn() => panic_test_function(), 'function panicked');
assert(panic_test_healthy() === 42);
