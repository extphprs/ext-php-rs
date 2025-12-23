<?php
// Test nested BailoutGuard cleanup - guards at multiple call stack levels

bailout_test_reset();

// This function creates 2 outer guards (id 1, 2) and calls an inner function
// that creates 2 inner guards (id 10, 11), then triggers exit().
// All 4 guards should be cleaned up.
bailout_test_nested(function() {
    exit(0);
});

// After the function returns, check that all 4 destructors were called
// (2 outer + 2 inner = 4 total, contributing 1+2+10+11 = 24 to counter)
$counter = bailout_test_get_counter();
assert($counter === 4, "Expected 4 destructors for nested test, got $counter");
