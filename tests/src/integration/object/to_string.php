<?php

require __DIR__ . '/../_utils.php';

class StringableOk
{
    public function __toString(): string
    {
        return 'hello';
    }
}

class StringableThrows
{
    public function __toString(): string
    {
        throw new \RuntimeException('nope');
    }
}

class NotStringableAtAll
{
    public int $x = 1;
}

assert(test_object_to_string(new StringableOk()) === 'hello');

// The original exception must survive the round trip: class, message and all.
// Before the fix this arrived as a generic \Exception wrapping a Debug dump.
try {
    test_object_to_string(new StringableThrows());
    throw new Exception('expected __toString to throw');
} catch (\RuntimeException $e) {
    assert('nope' === $e->getMessage());
}

// A class without __toString used to segfault in release builds, because
// zend_call_known_function only asserts a non-null handler under ZEND_DEBUG.
assert_exception_thrown(fn() => test_object_to_string(new NotStringableAtAll()));
