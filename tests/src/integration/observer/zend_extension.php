<?php

// ============================================================================
// Test 1: activate() was called during request startup
// ============================================================================

$activate_count = zend_ext_test_get_activate_count();
assert($activate_count >= 1, "Expected activate() to be called at least once, got: " . $activate_count);

// ============================================================================
// Test 2: op_array_handler() fires after compilation
// ============================================================================
// In CLI mode the entire file is compiled before execution, so op_array_handler
// has already been called for the main script and any functions defined above.

$op_count = zend_ext_test_get_op_array_handler_count();
assert($op_count > 0, "Expected op_array_handler to be called at least once, got: " . $op_count);

// ============================================================================
// Test 3: statement_handler() fires for executed statements
// ============================================================================

zend_ext_test_reset_statement_count();
$a = 1;
$b = 2;
$c = $a + $b;
$stmt_count = zend_ext_test_get_statement_count();
// The reset call, three assignments, and the get call each produce
// ZEND_EXT_STMT opcodes, so the count must be greater than zero.
assert($stmt_count > 0, "Expected statement_handler to be called, got: " . $stmt_count);

// ============================================================================
// Test 4: fcall_begin_handler() / fcall_end_handler() fire around calls
// ============================================================================
// ZEND_EXT_FCALL_BEGIN/END opcodes are emitted at call-sites in the calling
// code.  The reset function's own EXT_FCALL_BEGIN fires *before* the reset
// while its EXT_FCALL_END fires *after*, so begin and end may differ by one.
// We therefore only assert a minimum count for each.

function zend_ext_test_user_fn(): int
{
    return 42;
}

zend_ext_test_reset_fcall_counts();
zend_ext_test_user_fn();
zend_ext_test_user_fn();
zend_ext_test_user_fn();
$begin_count = zend_ext_test_get_fcall_begin_count();
$end_count = zend_ext_test_get_fcall_end_count();
assert($begin_count >= 3, "Expected at least 3 fcall_begin, got: " . $begin_count);
assert($end_count >= 3, "Expected at least 3 fcall_end, got: " . $end_count);

// ============================================================================
// Test 5: Nested user function calls are tracked
// ============================================================================

function zend_ext_outer(): int
{
    return zend_ext_inner();
}

function zend_ext_inner(): int
{
    return 99;
}

zend_ext_test_reset_fcall_counts();
$result = zend_ext_outer();
assert($result === 99, "Nested call should return 99, got: " . $result);

$nested_begin = zend_ext_test_get_fcall_begin_count();
$nested_end = zend_ext_test_get_fcall_end_count();
// outer() calls inner(), each with EXT_FCALL_BEGIN/END at the call-site.
// The call to outer() in the main script also generates EXT_FCALL, and
// inner()'s call-site inside outer() generates another pair.
assert($nested_begin >= 2, "Expected at least 2 nested fcall_begin, got: " . $nested_begin);
assert($nested_end >= 2, "Expected at least 2 nested fcall_end, got: " . $nested_end);

// ============================================================================
// Test 6: statement_handler counts increase with more statements
// ============================================================================

zend_ext_test_reset_statement_count();
$x = 1;
$y = 2;
$first_batch = zend_ext_test_get_statement_count();

zend_ext_test_reset_statement_count();
$x = 1;
$y = 2;
$z = 3;
$w = 4;
$second_batch = zend_ext_test_get_statement_count();

assert(
    $second_batch > $first_batch,
    "More statements should produce a higher count: first=$first_batch, second=$second_batch",
);
