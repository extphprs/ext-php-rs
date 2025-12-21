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
assert(test_defaults_nullable_with_some_default() === 'fallback', 'Should return fallback when called without arguments');
assert(test_defaults_nullable_with_some_default(null) === null, 'Should return null when null is passed');
assert(test_defaults_nullable_with_some_default('custom') === 'custom', 'Should return custom value when provided');
