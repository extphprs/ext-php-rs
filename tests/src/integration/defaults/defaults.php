<?php

assert(test_defaults_integer() === 42);
assert(test_defaults_integer(12) === 12);
assert(test_defaults_nullable_string() === null);
assert(test_defaults_nullable_string('test') === 'test');
assert(test_defaults_multiple_option_arguments() === 'Default');
assert(test_defaults_multiple_option_arguments(a: 'a') === 'a');
assert(test_defaults_multiple_option_arguments(b: 'b') === 'b');

// Test that passing null to a non-nullable parameter with a default value throws TypeError
// (fixes: https://github.com/extphprs/ext-php-rs/issues/538)
$threw = false;
try {
    test_defaults_integer(null);
} catch (TypeError $e) {
    $threw = true;
}
assert($threw, 'Expected TypeError when passing null to non-nullable parameter with default');

// But passing null to a nullable parameter should still work
assert(test_defaults_nullable_string(null) === null);

// Test nullable parameter with Some() default value
assert(
    test_defaults_nullable_with_some_default() === 'fallback',
    'Should return fallback when called without arguments'
);
assert(test_defaults_nullable_with_some_default(null) === null, 'Should return null when null is passed');
assert(test_defaults_nullable_with_some_default('custom') === 'custom', 'Should return custom value when provided');

function default_of(string $function): mixed
{
    return ( new ReflectionFunction($function) )->getParameters()[0]->getDefaultValue();
}

assert(default_of('test_defaults_integer') === 42);
assert(default_of('test_defaults_nullable_string') === null);
assert(default_of('test_defaults_multiple_option_arguments') === null);
assert(
    ( new ReflectionFunction('test_defaults_multiple_option_arguments') )->getParameters()[1]->getDefaultValue()
    === null
);
assert(default_of('test_defaults_nullable_with_some_default') === 'fallback');
assert(test_defaults_str() === 'HI');
assert(test_defaults_str('yo') === 'YO');
assert(default_of('test_defaults_str') === 'hi');
assert(test_defaults_float() === 3.0);
assert(default_of('test_defaults_float') === 1.5);

function required_of(string $method): int
{
    return ( new ReflectionMethod(OptionalArgs::class, $method) )->getNumberOfRequiredParameters();
}

assert(required_of('__construct') === 1);
assert(required_of('repeat') === 0);
assert(required_of('pick') === 1);
$args = new OptionalArgs('a');
assert($args->repeat() === 'aa');
assert($args->repeat(2, 3) === 'aaaaaa');
assert(( new OptionalArgs('a', 'b') )->pick(1) === 'abSome(1)None');
