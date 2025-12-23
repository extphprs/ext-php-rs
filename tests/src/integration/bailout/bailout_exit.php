<?php
// Test that Rust destructors are called when bailout occurs (issue #537)

bailout_test_reset();

// This function creates 3 DropTrackers and then calls a callback.
// The callback triggers exit(), which causes a bailout.
// Thanks to the try_catch wrapper, the Rust destructors should run
// when the function returns, incrementing the counter to 3.
bailout_test_with_callback(function() {
    exit(0);
});

// After the function returns, check that all 3 destructors were called
$counter = bailout_test_get_counter();
assert($counter === 3, "Expected 3 destructors to be called after bailout, got $counter");
