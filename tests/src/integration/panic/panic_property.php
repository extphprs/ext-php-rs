<?php

require __DIR__ . '/_panic_utils.php';

$obj = new PanicClass(false);
expect_rust_panic(fn() => $obj->boom, 'getter panicked');
expect_rust_panic(function () use ($obj) {
    $obj->boom = 1;
}, 'setter panicked');
expect_rust_panic(fn() => isset($obj->boom), 'getter panicked');
expect_rust_panic(fn() => (array) $obj, 'getter panicked');
assert($obj->plain === 7);
assert($obj->healthy() === 7);
