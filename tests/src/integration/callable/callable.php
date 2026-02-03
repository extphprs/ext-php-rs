<?php

// Basic callable test
assert(test_callable(fn (string $a) => $a, 'test') === 'test');

// Named arguments test - order should not matter, args matched by name
$namedResult = test_callable_named(fn (string $a, string $b) => "$a-$b");
assert($namedResult === 'first-second', "Named args failed: expected 'first-second', got '$namedResult'");

// Mixed positional + named arguments test
$mixedResult = test_callable_mixed(fn (string $pos, string $named) => "$pos|$named");
assert($mixedResult === 'positional|named_value', "Mixed args failed: expected 'positional|named_value', got '$mixedResult'");

// Macro test with named arguments only
$macroNamedResult = test_callable_macro_named(fn (string $x, string $y) => "$x $y");
assert($macroNamedResult === 'hello world', "Macro named args failed: expected 'hello world', got '$macroNamedResult'");

// Macro test with positional + named arguments
$macroMixedResult = test_callable_macro_mixed(fn (string $first, string $second) => "$first,$second");
assert($macroMixedResult === 'first,second_val', "Macro mixed args failed: expected 'first,second_val', got '$macroMixedResult'");

// Test with built-in PHP function using named args via ZendCallable::try_from_name
// This tests str_replace with named arguments in a different order
