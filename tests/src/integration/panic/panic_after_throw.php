<?php

try {
    panic_test_after_throw();
    assert(false, 'nothing was thrown');
} catch (\TypeError $e) {
    assert($e->getMessage() === 'thrown first', $e->getMessage());
    assert($e->getPrevious() === null);
}
assert(panic_test_healthy() === 42);
