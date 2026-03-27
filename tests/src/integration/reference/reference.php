<?php

// Test passing references to Rust functions (issue #102)

// Test string reference
$val = "hello";
$ref = &$val;
assert(test_ref_string($ref) === "hello", "String reference should work");

// Test modifying through reference
$val = "world";
assert(test_ref_string($ref) === "world", "Modified string reference should work");

// Test long reference
$num = 42;
$refNum = &$num;
assert(test_ref_long($refNum) === 42, "Long reference should work");

// Test modifying long through reference
$num = 100;
assert(test_ref_long($refNum) === 100, "Modified long reference should work");

// Test double reference
$dbl = 3.14;
$refDbl = &$dbl;
assert(test_ref_double($refDbl) === 3.14, "Double reference should work");

// Test modifying double through reference
$dbl = 2.71;
assert(test_ref_double($refDbl) === 2.71, "Modified double reference should work");

// Test &mut Zval - read
$str = "test";
assert(test_mut_zval($str) === "string: test", "&mut Zval should read string");

// Test &mut Zval - modify string in place
$str = "original";
test_mut_zval_set_string($str);
assert($str === "modified by rust", "&mut Zval should modify string in place");

// Test &mut Zval - modify to long in place
$str = "not a long";
test_mut_zval_set_long($str);
assert($str === 999, "&mut Zval should modify to long in place");

echo "Reference tests passed!";
