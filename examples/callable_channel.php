<?php
/**
 * Example PHP script demonstrating the callable_channel extension.
 *
 * This script shows how to use the CSP (Communicating Sequential Processes)
 * pattern to safely call PHP functions and closures from background threads.
 *
 * To run this example:
 * 1. Build the extension: cargo build --example callable_channel
 * 2. Run with PHP: php -d extension=./target/debug/examples/libcallable_channel.so callable_channel.php
 */

echo "=== Callable Channel Example ===\n\n";

// Test 1: Synchronous function call via channel
echo "Test 1: Synchronous function call (strtoupper)\n";
$result = call_function_sync('strtoupper', ['hello world']);
echo "  strtoupper('hello world') = '$result'\n\n";

// Test 2: Register and call a closure synchronously
echo "Test 2: Register and call closure synchronously\n";
$adder = fn($a, $b) => $a + $b;
$closureId = register_callback($adder);
echo "  Registered closure with ID: $closureId\n";
echo "  Registered closure count: " . registered_closure_count() . "\n";

$result = call_closure_sync($closureId, [10, 20]);
echo "  adder(10, 20) = $result\n\n";

// Test 3: Async function calls from background threads
echo "Test 3: Async function calls from background threads\n";
call_function_async('strtoupper', ['async test 1']);
call_function_async('strtolower', ['ASYNC TEST 2']);
call_function_async('strlen', ['count me']);

echo "  Queued 3 async function calls\n";
echo "  Pending callbacks: " . pending_callback_count() . "\n";

// Small delay to let threads queue the calls
usleep(10000);

echo "  Processing pending callbacks...\n";
$processed = process_callbacks();
echo "  Processed $processed callbacks\n\n";

// Test 4: Async closure calls
echo "Test 4: Async closure calls\n";
$multiplier = fn($x) => $x * 2;
$multiplierId = register_callback($multiplier);
echo "  Registered multiplier closure with ID: $multiplierId\n";

call_closure_async($multiplierId, [5]);
call_closure_async($multiplierId, [10]);
call_closure_async($multiplierId, [15]);

usleep(10000);
echo "  Pending callbacks: " . pending_callback_count() . "\n";
$processed = process_callbacks();
echo "  Processed $processed callbacks\n\n";

// Test 5: Parallel closure calls
echo "Test 5: Parallel closure calls from multiple threads\n";
$summer = fn($x) => "Processed: $x";
$summerId = register_callback($summer);

$values = [1, 2, 3, 4, 5];
$queued = parallel_closure_calls($summerId, $values);
echo "  Queued $queued parallel calls\n";

usleep(50000); // Wait for threads to complete queueing
echo "  Pending callbacks: " . pending_callback_count() . "\n";
$processed = process_callbacks();
echo "  Processed $processed callbacks\n\n";

// Test 6: Stateful closure (demonstrates closure captures)
echo "Test 6: Stateful closure with captured variable\n";
$counter = 0;
$incrementer = function() use (&$counter) {
    $counter++;
    return $counter;
};
$incrementerId = register_callback($incrementer);

echo "  Initial counter: $counter\n";
for ($i = 0; $i < 3; $i++) {
    call_closure_sync($incrementerId, []);
}
echo "  Counter after 3 calls: $counter\n\n";

// Cleanup
echo "Cleanup:\n";
$unregistered = 0;
if (unregister_callback($closureId)) $unregistered++;
if (unregister_callback($multiplierId)) $unregistered++;
if (unregister_callback($summerId)) $unregistered++;
if (unregister_callback($incrementerId)) $unregistered++;
echo "  Unregistered $unregistered closures\n";
echo "  Remaining registered closures: " . registered_closure_count() . "\n";

echo "\n=== All tests completed ===\n";
