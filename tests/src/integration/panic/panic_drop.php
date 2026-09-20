<?php

$warnings = [];
set_error_handler(function (int $errno, string $message) use (&$warnings): bool {
    $warnings[] = [$errno, $message];
    return true;
});

$obj = new PanicDrop();
unset($obj);

restore_error_handler();
assert(count($warnings) === 1, var_export($warnings, true));
assert($warnings[0][0] === E_WARNING);
assert(str_contains($warnings[0][1], 'Rust panic in Drop for PanicDrop: drop panicked'), $warnings[0][1]);
assert(panic_test_healthy() === 42);
