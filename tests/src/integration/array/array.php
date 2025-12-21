<?php

// Tests sequential arrays
$array = test_array(['a', 'b', 'c', 'd']);
unset($array[2]);

assert(is_array($array));
assert(count($array) === 3);
assert(in_array('a', $array));
assert(in_array('b', $array));
assert(in_array('d', $array));

// Tests associative arrays
$assoc = test_array_assoc([
    'a' => '1',
    'b' => '2',
    'c' => '3'
]);

assert(array_key_exists('a', $assoc));
assert(array_key_exists('b', $assoc));
assert(array_key_exists('c', $assoc));
assert(in_array('1', $assoc));
assert(in_array('2', $assoc));
assert(in_array('3', $assoc));

$arrayKeys = test_array_keys();
assert($arrayKeys[-42] === "foo");
assert($arrayKeys[0] === "bar");
assert($arrayKeys[5] === "baz");
assert($arrayKeys[10] === "qux");
assert($arrayKeys["10"] === "qux");
assert($arrayKeys["quux"] === "quuux");

$assoc_keys = test_array_assoc_array_keys([
    'a' => '1',
    2 => '2',
    '3' => '3',
]);
assert($assoc_keys === [
    'a' => '1',
    2 => '2',
    '3' => '3',
]);
$assoc_keys = test_btree_map([
    'a' => '1',
    2 => '2',
    '3' => '3',
]);
assert($assoc_keys === [
    2 => '2',
    '3' => '3',
    'a' => '1',
]);

$assoc_keys = test_array_assoc_array_keys(['foo', 'bar', 'baz']);
assert($assoc_keys === [
    0 => 'foo',
    1 => 'bar',
    2 => 'baz',
]);
assert(test_btree_map(['foo', 'bar', 'baz']) === [
    0 => 'foo',
    1 => 'bar',
    2 => 'baz',
]);

$leading_zeros = test_array_assoc_array_keys([
  '0' => 'zero',
  '00' => 'zerozero',
  '007' => 'bond',
]);

assert(array_key_exists(0, $leading_zeros), '"0" should become integer key 0');
assert($leading_zeros[0] === 'zero', 'Value at key 0 should be "zero"');

assert(array_key_exists('007', $leading_zeros), '"007" should stay as string key');
assert($leading_zeros['007'] === 'bond', 'Value at key "007" should be "bond"');

assert(array_key_exists('00', $leading_zeros), '"00" should stay as string key');
assert($leading_zeros['00'] === 'zerozero', 'Value at key "00" should be "zerozero"');

// Test Option<&ZendHashTable> with literal array (issue #515)
// This should work without "could not be passed by reference" error
assert(test_optional_array_ref([1, 2, 3]) === 3, 'Option<&ZendHashTable> should accept literal array');
assert(test_optional_array_ref(null) === -1, 'Option<&ZendHashTable> should accept null');
$arr = ['a', 'b', 'c', 'd'];
assert(test_optional_array_ref($arr) === 4, 'Option<&ZendHashTable> should accept variable array');

// Test Option<&mut ZendHashTable> (anti-regression for issue #515)
$mut_arr = ['x', 'y'];
assert(test_optional_array_mut_ref($mut_arr) === 3, 'Option<&mut ZendHashTable> should accept variable array and add element');
assert(array_key_exists('added_by_rust', $mut_arr), 'Rust should have added a key to the array');
assert($mut_arr['added_by_rust'] === 'value', 'Added value should be correct');
$null_arr = null;
assert(test_optional_array_mut_ref($null_arr) === -1, 'Option<&mut ZendHashTable> should accept null');
