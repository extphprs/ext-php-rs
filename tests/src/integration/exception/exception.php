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

try {
    call_throwing_callable(function () {
        throw new \LogicException('boom');
    });
    assert(false, 'the original exception should have propagated');
} catch (\Throwable $e) {
    assert($e instanceof \LogicException, get_class($e));
    assert('boom' === $e->getMessage(), $e->getMessage());
    assert(__FILE__ === $e->getFile(), $e->getFile());
    assert(
        '{closure}' === $e->getTrace()[0]['function'] || str_contains($e->getTrace()[0]['function'], 'closure'),
        $e->getTrace()[0]['function']
    );
}

try {
    throw_over_pending_exception(function () {
        throw new \RangeException('first');
    });
    assert(false, 'the pending exception should have propagated');
} catch (\Throwable $e) {
    assert($e instanceof \RangeException, get_class($e));
    assert('first' === $e->getMessage(), $e->getMessage());
}

try {
    throw_non_object();
    assert(false, 'throwing a non-object should have failed');
} catch (\Throwable $e) {
    assert('an object was expected' === $e->getMessage(), $e->getMessage());
}

try {
    throw_interface_class();
    assert(false, 'an Error should have been thrown instead of the interface');
} catch (\Throwable $e) {
    assert(get_class($e) === \Error::class, get_class($e));
    assert(str_contains($e->getMessage(), 'cannot throw Throwable'), $e->getMessage());
    assert(str_contains($e->getMessage(), 'original message: cannot instantiate'), $e->getMessage());
}

try {
    throw_nul_message();
    assert(false, 'an Error should have been thrown instead of the NUL message');
} catch (\Throwable $e) {
    assert(get_class($e) === \Error::class, get_class($e));
    assert(str_contains($e->getMessage(), 'original message: before\\0after'), $e->getMessage());
}

// `#[php(sensitive(...))]` marks parameters `#[\SensitiveParameter]`, so traces hide their values.
if (PHP_VERSION_ID >= 80200) {
    ini_set('zend.exception_ignore_args', '0');

    try {
        throw_with_password('admin', 'hunter2');
        assert(false, 'throw_with_password should throw');
    } catch (\Exception $e) {
        $args = $e->getTrace()[0]['args'];
        assert($args[0] === 'admin', 'An unmarked argument is kept');
        assert($args[1] instanceof \SensitiveParameterValue, 'A sensitive function argument is hidden');
        assert($args[1]->getValue() === 'hunter2');
        assert(!str_contains($e->getTraceAsString(), 'hunter2'), $e->getTraceAsString());
    }

    try {
        new Test\TestCredentials('s3cret');
        assert(false, 'the constructor should throw');
    } catch (\Exception $e) {
        $args = $e->getTrace()[0]['args'];
        assert($args[0] instanceof \SensitiveParameterValue, 'A sensitive method argument is hidden');
        assert(!str_contains($e->getTraceAsString(), 's3cret'), $e->getTraceAsString());
    }

    $parameters = (new \ReflectionMethod(Test\TestCredentials::class, '__construct'))->getParameters();
    assert($parameters[0]->getAttributes(\SensitiveParameter::class) !== [], 'Reflection sees the attribute');
}

