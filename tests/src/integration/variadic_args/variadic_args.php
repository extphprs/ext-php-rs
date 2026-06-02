<?php

require __DIR__ . '/../_utils.php';

$a = 'a';
$b = 'b';
$c = 'c';

$array = [$b, 'a', 'c'];

// Passing arguments as references
$args = test_variadic_args();
assert($args === [], 'Expected no arguments to be returned');

$args = test_variadic_args($a);
assert($args === ['a'], 'Expected to return argument $a');

$args = test_variadic_args($a, $b, $c);
assert($args === ['a', 'b', 'c'], 'Expected to return arguments $a, $b and $c');

$args = test_variadic_args(...$array);
assert($args === ['b', 'a', 'c'], 'Expected to return an array with the array $array');

assert_exception_thrown('test_variadic_add_required');

// Values directly passed
$sum = test_variadic_add_required(1, 2, 3); // 1
assert($sum === 6, 'Expected to return 6');

$count = test_variadic_add_required(11); // 11
assert($count === 11, 'Allow only one argument');

$types = test_variadic_args('a', 1, ['abc', 'def', 0.01], true, new stdClass());
assert(gettype(end($types[2])) === 'double', 'Type of argument 2 and its last element should be a float of 0.01');
assert($types[3], 'Arg 4 should be boolean true');
assert($types[4] instanceof stdClass, 'Last argument is an instance of an StdClass');

// Test variadic count
assert(test_variadic_count() === 0, 'Empty variadic should return 0');
assert(test_variadic_count(1) === 1, 'Single arg should return 1');
assert(test_variadic_count(1, 2, 3, 4, 5) === 5, 'Five args should return 5');
assert(test_variadic_count(...range(1, 100)) === 100, 'Spread of 100 items should return 100');

// Test variadic types detection
$type_result = test_variadic_types(42, 'hello', 3.14, true, [1, 2], new stdClass(), null);
assert(
    $type_result === ['long', 'string', 'double', 'bool', 'array', 'object', 'null'],
    'Should detect all types correctly'
);

$type_result = test_variadic_types();
assert($type_result === [], 'Empty variadic should return empty array');

// Test variadic with string operations
$prefixed = test_variadic_strings('pre_', 'a', 'b', 'c');
assert($prefixed === ['pre_a', 'pre_b', 'pre_c'], 'Should prefix all strings');

$prefixed = test_variadic_strings('x_');
assert($prefixed === [], 'No suffixes should return empty');

$prefixed = test_variadic_strings('test_', 'one', 123, 'two');
assert(count($prefixed) === 2, 'Should only process strings, filtering out non-strings');

// Test variadic sum
assert(test_variadic_sum_all() === 0, 'Empty sum should be 0');
assert(test_variadic_sum_all(1, 2, 3, 4, 5) === 15, 'Sum 1-5 should be 15');
assert(test_variadic_sum_all(10, 20, 30) === 60, 'Sum should be 60');
assert(test_variadic_sum_all(-5, 5) === 0, 'Negative and positive should cancel');
assert(test_variadic_sum_all(100, 'not a number', 200) === 300, 'Should skip non-numeric values');

// Test variadic with optional parameter
assert(test_variadic_optional('req', null) === 'req-none-0', 'Optional null, no extras');
assert(test_variadic_optional('req', 42) === 'req-42-0', 'Optional provided, no extras');
assert(test_variadic_optional('req', 42, 'a', 'b') === 'req-42-2', 'Optional and extras provided');
assert(test_variadic_optional('req', null, 'x') === 'req-none-1', 'Null optional, one extra');

// Test variadic empty check
assert(test_variadic_empty_check() === true, 'No args should be empty');
assert(test_variadic_empty_check(1) === false, 'One arg should not be empty');
assert(test_variadic_empty_check(1, 2, 3) === false, 'Multiple args should not be empty');

// Test variadic first/last
$fl = test_variadic_first_last();
assert($fl === [], 'Empty should return empty array');

$fl = test_variadic_first_last(1);
assert($fl === [1], 'Single item should return just that item');

$fl = test_variadic_first_last(1, 2);
assert($fl === [1, 2], 'Two items should return first and last');

$fl = test_variadic_first_last(1, 2, 3, 4, 5);
assert($fl === [1, 5], 'Multiple items should return first and last');

$fl = test_variadic_first_last('start', true, 3.14, ['middle'], 'end');
assert($fl[0] === 'start' && $fl[1] === 'end', 'Should preserve types for first and last');
