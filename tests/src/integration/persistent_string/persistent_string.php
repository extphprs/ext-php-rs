<?php
$result = test_persistent_string();
assert($result === "persistent string test passed", "Persistent string test failed: $result");

$result = test_non_persistent_string();
assert($result === "non-persistent string test passed", "Non-persistent string test failed: $result");

$result = test_persistent_string_read();
assert($result === "read: READ_BEFORE_DROP", "Persistent string read test failed: $result");

$result = test_persistent_string_loop(100);
assert($result === "completed 100 iterations", "Persistent string loop test failed: $result");

$result = test_interned_string_persistent();
assert($result === "interned persistent test passed", "Interned persistent string test failed: $result");

$result = test_interned_string_non_persistent();
assert($result === "interned non-persistent test passed", "Interned non-persistent string test failed: $result");
