<?php

require __DIR__ . '/_panic_utils.php';

expect_rust_panic(fn() => panic_test_guard(), 'guarded panic');
assert(panic_test_guard_drops() === 2, (string) panic_test_guard_drops());
assert(panic_test_healthy() === 42);
