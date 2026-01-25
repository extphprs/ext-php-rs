<?php

// Reset counters at the start
observer_test_reset();

// Define a user function to observe
function my_test_function(): string
{
    return "hello";
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

// Call user functions
my_test_function();
another_function(5);
my_test_function();

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
function outer(): int
{
    return inner();
}

function inner(): int
{
    return 42;
}

observer_test_reset();
$result = outer();
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
