<?php

// Regression coverage for a refcount leak in `#[php(prop)]` field getters
// holding owned refcounted types (e.g. `String`).
//
// Mechanic: the generated getter writes a fresh `zend_string` (refcount=1)
// into the `rv` slot via `set_zval`. PHP's `Exception::getMessage` reads it
// with `zval_get_string(prop) + RETURN_STR`. `zval_get_string` addrefs to 2;
// `RETURN_STR` transfers the pointer to `return_value` without changing the
// refcount; the stack `rv` goes out of scope without `zval_ptr_dtor`. One
// refcount is orphaned per call, leaking the underlying zend_string.
//
// Detection: 500 calls grow the emalloc heap by ~75KB on release builds, and
// trigger `_zend_hash_str_add_or_update_i` assertions on debug builds. Either
// signal fails the test. Hosted in its own PHP file so a debug-build SIGABRT
// does not silently drop later assertions in class.php.
//
// Surfaced first in production by biscuit-php's `DatalogException` subclasses,
// which declare `#[php(prop, flags = Protected)] message: String` and shadow
// the parent `\Exception::$message`.

$first_call_returned = null;
$leak_observed = null;
try {
    throw_exception_with_message_prop();
} catch (TestExceptionMessageLeak $e) {
    $first_call_returned = $e->getMessage();
    // Warm up so any one-shot allocations (cache slots, opcode cache,
    // hashtable resizes) settle before measurement.
    for ($i = 0; $i < 16; $i++) {
        $e->getMessage();
    }
    gc_collect_cycles();
    $mem_before = memory_get_usage();
    for ($i = 0; $i < 500; $i++) {
        $e->getMessage();
    }
    gc_collect_cycles();
    $mem_after = memory_get_usage();
    $leak_observed = $mem_after - $mem_before;
}
assert(
    $first_call_returned === 'leak-bait message contents',
    'Sanity check: one getMessage() call should return the stored message verbatim. Got: '
        . var_export($first_call_returned, true)
);
assert(
    $leak_observed !== null && $leak_observed < 8192,
    'getMessage() on a #[php(prop)] String field leaks zend_strings: 500 '
    . 'calls grew the emalloc heap by '
    . var_export($leak_observed, true)
    . ' bytes. Each call orphans one zend_string refcount (~150 bytes); '
    . 'with the fix this delta should be near zero.'
);
