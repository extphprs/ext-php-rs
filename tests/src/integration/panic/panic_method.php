<?php

require __DIR__ . '/_panic_utils.php';

$obj = new PanicClass(false);
expect_rust_panic($obj->method(...), 'method panicked');
expect_rust_panic(PanicClass::staticMethod(...), 'static method panicked');
expect_rust_panic($obj->ifaceMethod(...), 'interface method panicked');
assert($obj instanceof PanicIface);
assert($obj->healthy() === 7);
