<?php

require __DIR__ . '/../_utils.php';

assert_exception_thrown(fn() => throw_default_exception(), \Exception::class);

try {
    throw_custom_exception();
} catch (\Throwable $e) {
    // Check if object is initiated
    assert($e instanceof \Test\TestException);
    assert('Not good custom!' === $e->getMessage());
}
