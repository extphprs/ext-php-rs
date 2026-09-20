<?php

require __DIR__ . '/_panic_utils.php';

expect_rust_panic(fn() => panic_test_payload(), 'non-string panic payload');
assert(panic_test_healthy() === 42);
