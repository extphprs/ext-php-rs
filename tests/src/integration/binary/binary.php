<?php

$bin = test_binary(pack('L*', 1, 2, 3, 4, 5));
$result = unpack('L*', $bin);

assert(count($result) === 5);
assert(in_array(1, $result));
assert(in_array(2, $result));
assert(in_array(3, $result));
assert(in_array(4, $result));
assert(in_array(5, $result));

// Regression for #729: returning a Binary whose packed length is 0 or 1
// must not corrupt the Zend heap on zval destruction.
$empty_u8 = test_binary_empty_u8();
assert(is_string($empty_u8));
assert(strlen($empty_u8) === 0);

$single_u8 = test_binary_single_u8();
assert(is_string($single_u8));
assert(strlen($single_u8) === 1);
assert(ord($single_u8) === 42);

$empty_u32 = test_binary_empty_u32();
assert(is_string($empty_u32));
assert(strlen($empty_u32) === 0);
