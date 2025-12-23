<?php
// Test BailoutGuard - ensures values wrapped in BailoutGuard are cleaned up on bailout

bailout_test_reset();

// This function creates 2 guarded trackers and 1 unguarded tracker,
// then calls a callback that triggers exit().
// All 3 should be cleaned up (guarded ones via BailoutGuard, unguarded via try_call).
bailout_test_with_guard(function() {
    exit(0);
});

// After the function returns, check that all 3 destructors were called
$counter = bailout_test_get_counter();
assert($counter === 3, "Expected 3 destructors to be called with BailoutGuard, got $counter");
