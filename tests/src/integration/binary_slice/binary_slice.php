<?php

$packed = pack('P*', 1, 2, 3, 4, 5);
assert(test_binary_slice_sum($packed) === 15);
assert(test_binary_slice_len($packed) === 5);

assert(test_binary_slice_len('') === 0);
assert(test_binary_slice_sum(pack('P', PHP_INT_MAX)) === PHP_INT_MAX);

try {
    test_binary_slice_len('1234567');
    assert(false, 'a 7-byte string must be rejected, not truncated');
} catch (Exception $e) {
}
