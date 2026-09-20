<?php

require __DIR__ . '/_panic_utils.php';

expect_rust_panic(fn() => panic_test_nested(fn() => panic_test_function()), 'function panicked');
assert(panic_test_nested_result() === 'pending: Error', panic_test_nested_result());
assert(panic_test_healthy() === 42);
