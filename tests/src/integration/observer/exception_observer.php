<?php

exception_observer_test_reset();

// Test 1: Initial state
$initial_count = exception_observer_test_get_count();
assert($initial_count === 0, "Initial exception count should be 0, got: " . $initial_count);

// Test 2: Throw and catch an exception
try {
    throw new RuntimeException("Test exception message", 42);
} catch (RuntimeException $e) {
    // Exception was caught
}

$count = exception_observer_test_get_count();
assert($count === 1, "Expected 1 exception, got: " . $count);

$last_class = exception_observer_test_get_last_class();
assert($last_class === "RuntimeException", "Expected class 'RuntimeException', got: " . $last_class);

$last_message = exception_observer_test_get_last_message();
assert($last_message === "Test exception message", "Expected message 'Test exception message', got: " . $last_message);

$last_code = exception_observer_test_get_last_code();
assert($last_code === 42, "Expected code 42, got: " . $last_code);

$last_line = exception_observer_test_get_last_line();
assert($last_line === 11, "Expected line 11, got: " . $last_line);

// Test 3: Reset and verify
exception_observer_test_reset();

$count = exception_observer_test_get_count();
assert($count === 0, "After reset, count should be 0, got: " . $count);

// Test 4: Multiple exceptions
try {
    throw new InvalidArgumentException("First");
} catch (Exception $e) {}

try {
    throw new LogicException("Second");
} catch (Exception $e) {}

try {
    throw new Exception("Third", 100);
} catch (Exception $e) {}

$count = exception_observer_test_get_count();
assert($count === 3, "Expected 3 exceptions, got: " . $count);

$last_class = exception_observer_test_get_last_class();
assert($last_class === "Exception", "Last class should be 'Exception', got: " . $last_class);

$last_message = exception_observer_test_get_last_message();
assert($last_message === "Third", "Last message should be 'Third', got: " . $last_message);

$last_code = exception_observer_test_get_last_code();
assert($last_code === 100, "Last code should be 100, got: " . $last_code);

// Test 5: Nested exceptions
exception_observer_test_reset();

function throwNested() {
    throw new RuntimeException("Nested exception");
}

try {
    throwNested();
} catch (Exception $e) {}

$count = exception_observer_test_get_count();
assert($count === 1, "Expected 1 nested exception, got: " . $count);

$last_class = exception_observer_test_get_last_class();
assert($last_class === "RuntimeException", "Expected 'RuntimeException', got: " . $last_class);

// Test 6: Backtrace capture
exception_observer_test_reset();

function innerThrow() {
    throw new RuntimeException("From inner");
}

function outerCall() {
    innerThrow();
}

try {
    outerCall();
} catch (Exception $e) {}

$backtrace_depth = exception_observer_test_get_backtrace_depth();
assert($backtrace_depth >= 2, "Expected backtrace depth >= 2, got: " . $backtrace_depth);

$backtrace_functions = exception_observer_test_get_backtrace_functions();
assert(strpos($backtrace_functions, "innerThrow") !== false, "Expected 'innerThrow' in backtrace, got: " . $backtrace_functions);
assert(strpos($backtrace_functions, "outerCall") !== false, "Expected 'outerCall' in backtrace, got: " . $backtrace_functions);
