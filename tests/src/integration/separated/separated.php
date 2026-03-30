<?php

// ============================================================
// Separated: accepts literals, does NOT modify caller's value
// ============================================================

// Accepts a literal array (no & required — this was the core bug)
assert(
    test_separated_array_push([1, 2, 3]) === 4,
    'Separated should accept literal array and return new length after push'
);

// Caller's variable is NOT modified (COW separation)
$arr = ['a', 'b'];
$len = test_separated_array_push($arr);
assert($len === 3, 'Separated push should report 3 elements');
assert(count($arr) === 2, 'Caller array must be unchanged after Separated mutation');
assert(!array_key_exists(2, $arr), 'No new key should appear in caller array');

// Read-only access works
assert(test_separated_array_len([10, 20, 30]) === 3, 'Separated read should report correct length');
assert(test_separated_array_len([]) === 0, 'Separated should handle empty array');

// Works with different PHP types
assert(test_separated_string("hello") === "hello", 'Separated should read strings');
assert(test_separated_long(42) === 42, 'Separated should read integers');

// Passthrough returns a copy of the value
$original = [1, 2, 3];
$copy = test_separated_passthrough($original);
assert($copy === [1, 2, 3], 'Passthrough should return equivalent value');

// Type reporting
assert(test_separated_type(42) === "Long", 'Separated should report Long type');
assert(test_separated_type("str") === "String", 'Separated should report String type');
assert(test_separated_type([1]) === "Array", 'Separated should report Array type');
assert(test_separated_type(null) === "Null", 'Separated should report Null type');
assert(test_separated_type(true) === "True", 'Separated should report True type');
assert(test_separated_type(3.14) === "Double", 'Separated should report Double type');

// ============================================================
// PhpRef: requires &$var, DOES modify caller's value
// ============================================================

// Increment modifies the caller's variable
$x = 5;
test_phpref_increment($x);
assert($x === 6, 'PhpRef increment should modify caller variable to 6');

test_phpref_increment($x);
assert($x === 7, 'PhpRef increment should modify caller variable to 7');

// Array push modifies the caller's array
$arr = ['a', 'b'];
$len = test_phpref_array_push($arr);
assert($len === 3, 'PhpRef push should report 3 elements');
assert(count($arr) === 3, 'Caller array should now have 3 elements');
assert(in_array('pushed_by_rust', $arr), 'Caller array should contain pushed value');

// Read through PhpRef
$val = 99;
assert(test_phpref_read($val) === 99, 'PhpRef read should return the value');

// Replace value through PhpRef
$val = 42;
test_phpref_set_string($val);
assert($val === "replaced_by_rust", 'PhpRef should replace the value entirely');
assert(is_string($val), 'Replaced value should be a string');

// PhpRef rejects literals (this should fail — uncomment to verify manually)
// test_phpref_increment(42); // Fatal error: cannot pass by reference
