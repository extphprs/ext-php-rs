<?php

error_observer_test_reset();

$initial_error_count = error_observer_test_get_error_count();
$initial_warning_count = error_observer_test_get_warning_count();

assert($initial_error_count === 0, "Initial error count should be 0, got: " . $initial_error_count);
assert($initial_warning_count === 0, "Initial warning count should be 0, got: " . $initial_warning_count);

trigger_error("Test warning message", E_USER_WARNING);

$warning_count = error_observer_test_get_warning_count();
assert($warning_count === 1, "Expected 1 warning, got: " . $warning_count);

$last_message = error_observer_test_get_last_message();
assert(str_contains($last_message, "Test warning message"), "Message should contain 'Test warning message', got: " . $last_message);

$last_line = error_observer_test_get_last_line();
assert($last_line === 11, "Line should be 11, got: " . $last_line);

error_observer_test_reset();

trigger_error("Warning 1", E_USER_WARNING);
trigger_error("Warning 2", E_USER_WARNING);
trigger_error("Warning 3", E_USER_WARNING);

$warning_count = error_observer_test_get_warning_count();
assert($warning_count === 3, "Expected 3 warnings after multiple triggers, got: " . $warning_count);

$last_message = error_observer_test_get_last_message();
assert(str_contains($last_message, "Warning 3"), "Last message should be 'Warning 3', got: " . $last_message);

error_observer_test_reset();
$old_level = error_reporting(E_ALL);
@trigger_error("This notice should not be observed", E_USER_NOTICE);
error_reporting($old_level);

$warning_count = error_observer_test_get_warning_count();
$error_count = error_observer_test_get_error_count();
assert($warning_count === 0, "Notice should not be counted as warning, got: " . $warning_count);
assert($error_count === 0, "Notice should not be counted as error, got: " . $error_count);
