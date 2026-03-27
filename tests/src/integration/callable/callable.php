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

// Empty named params (should behave like try_call)
$emptyNamedResult = test_callable_empty_named(fn (string $a) => "got:$a");
assert($emptyNamedResult === 'got:hello', "Empty named args failed: expected 'got:hello', got '$emptyNamedResult'");

// Built-in PHP function with named args (str_replace with args in non-standard order)
$builtinResult = test_callable_builtin_named();
assert($builtinResult === 'Hello PHP', "Builtin named args failed: expected 'Hello PHP', got '$builtinResult'");

// Duplicate named params - last value wins
$dupResult = test_callable_duplicate_named(fn (string $a) => "val:$a");
assert($dupResult === 'val:overwritten', "Duplicate named args failed: expected 'val:overwritten', got '$dupResult'");

// === CachedCallable tests ===

// Basic cached callable
assert(test_cached_callable_basic(fn (string $a) => $a, 'cached') === 'cached');

// Repeated calls (sum of fn(i) => i*2 for i in 0..10 = 2*(0+1+...+9) = 90)
$repeatedResult = test_cached_callable_repeated(fn (int $i) => $i * 2);
assert($repeatedResult === 90, "Repeated cached calls failed: expected 90, got '$repeatedResult'");

// Cached callable with named arguments
$cachedNamedResult = test_cached_callable_named(fn (string $a, string $b) => "$a-$b");
assert($cachedNamedResult === 'first-second', "Cached named args failed: expected 'first-second', got '$cachedNamedResult'");

// Cached callable with mixed arguments
$cachedMixedResult = test_cached_callable_mixed(fn (string $pos, string $named) => "$pos|$named");
assert($cachedMixedResult === 'positional|named_value', "Cached mixed args failed: expected 'positional|named_value', got '$cachedMixedResult'");

// Exception recovery - cached callable stays valid after PHP exception
$exceptionResult = test_cached_callable_exception_recovery(function (bool $shouldThrow) {
    if ($shouldThrow) {
        throw new \RuntimeException('Test exception');
    }
    return 'recovered';
});
assert($exceptionResult === 'recovered', "Exception recovery failed: expected 'recovered', got '$exceptionResult'");

// Built-in function caching
$builtinResult = test_cached_callable_builtin();
assert($builtinResult === 5, "Cached builtin failed: expected 5, got '$builtinResult'");
