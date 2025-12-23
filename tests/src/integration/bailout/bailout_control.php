<?php
// Control test - verify destructors work without bailout

bailout_test_reset();

// Call function that creates DropTrackers without bailout
bailout_test_without_exit();

// The destructors should have been called
$counter = bailout_test_get_counter();
assert($counter === 2, "Expected 2 destructors to be called, got $counter");
