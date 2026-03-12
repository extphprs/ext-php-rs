<?php

// Reset counters at the start
observer_test_reset();

// Define a user function to observe
function my_test_function(string $suffix = ""): string
{
    return "hello{$suffix}";
}

// Define another user function
function another_function(int $x): int
{
    return $x * 2;
}

// Get initial counts (should be 0 after reset)
$initial_call_count = observer_test_get_call_count();
$initial_end_count = observer_test_get_end_count();

// Note: The observer functions themselves are internal (Rust) functions,
// so they should NOT be counted by our observer which filters to user functions only.

// Resolve callables at runtime so opcache cannot fold these calls away.
$user_calls = [
    [getenv('EXT_PHP_RS_OBSERVER_FN1') ?: 'my_test_function', ['']],
    [getenv('EXT_PHP_RS_OBSERVER_FN2') ?: 'another_function', [5]],
    [getenv('EXT_PHP_RS_OBSERVER_FN3') ?: 'my_test_function', [' again']],
];

$results = [];
foreach ($user_calls as [$function, $args]) {
    $results[] = $function(...$args);
}

assert(
    $results === ['hello', 10, 'hello again'],
    'Unexpected results from observed user function calls',
);

// Get counts after calling user functions
$call_count = observer_test_get_call_count();
$end_count = observer_test_get_end_count();

// We called 3 user functions, so we expect:
// - call_count should have increased by 3
// - end_count should have increased by 3
assert($call_count >= 3, "Expected at least 3 calls, got: " . $call_count);
assert($end_count >= 3, "Expected at least 3 ends, got: " . $end_count);
assert($call_count === $end_count, "Call count and end count should match");

// Test nested function calls
function outer(callable $callback): int
{
    return $callback();
}

function inner(): int
{
    return 42;
}

observer_test_reset();
$outer = getenv('EXT_PHP_RS_OUTER_FN') ?: 'outer';
$inner = getenv('EXT_PHP_RS_INNER_FN') ?: 'inner';
$result = $outer($inner);
assert($result === 42, "Nested call should return 42");

$nested_call_count = observer_test_get_call_count();
$nested_end_count = observer_test_get_end_count();

// outer() calls inner(), so we expect 2 calls
assert(
    $nested_call_count >= 2,
    "Expected at least 2 nested calls, got: " . $nested_call_count,
);
assert(
    $nested_end_count >= 2,
    "Expected at least 2 nested ends, got: " . $nested_end_count,
);
