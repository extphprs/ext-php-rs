<?php

require __DIR__ . '/_panic_utils.php';

$obj = null;
expect_rust_panic(function () use (&$obj) {
    $obj = new PanicClass(true);
}, 'constructor panicked');
assert($obj === null);
assert(( new PanicClass(false) )->healthy() === 7);
