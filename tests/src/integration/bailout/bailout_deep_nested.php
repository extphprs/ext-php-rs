<?php
// Test deeply nested BailoutGuard cleanup - 3 levels of nesting

bailout_test_reset();

// This function creates guards at 3 nesting levels:
// - Level 1: 1 guard
// - Level 2 (closure): 1 guard
// - Level 3 (nested closure): 1 guard
// Then triggers exit() at the deepest level.
// All 3 guards should be cleaned up.
bailout_test_deep_nested(function() {
    exit(0);
});

// After the function returns, check that all 3 destructors were called
$counter = bailout_test_get_counter();
assert($counter === 3, "Expected 3 destructors for deep nested test, got $counter");
