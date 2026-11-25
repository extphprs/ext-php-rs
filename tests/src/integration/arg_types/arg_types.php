<?php

// An untyped parameter has no type, as `function f($value)` declares it.
$untyped = new ReflectionFunction('test_untyped_echo');
$parameter = $untyped->getParameters()[0];
assert(!$parameter->hasType(), 'an untyped parameter has no type');
assert($parameter->getType() === null);
assert(!$untyped->hasReturnType());
assert(test_untyped_echo([1, 2]) === [1, 2]);

// A union of scalar types.
$twice = new ReflectionFunction('test_number_twice');
$type = $twice->getParameters()[0]->getType();
assert($type instanceof ReflectionUnionType, get_debug_type($type));
assert((string) $type === 'int|float', (string) $type);
assert((string) $twice->getReturnType() === 'int|float', (string) $twice->getReturnType());
assert(test_number_twice(2) === 4);
assert(test_number_twice(1.5) === 3.0);

// `null` in a union makes it nullable.
$nullable = new ReflectionFunction('test_nullable_union');
$type = $nullable->getParameters()[0]->getType();
assert($type->allowsNull());
assert((string) $type === 'string|int|null', (string) $type);

// PHP's inheritance checks see the declared types: an untyped parent parameter
// accepts any override of its own, and an `int|float` parameter may be widened
// and its `int|float` return narrowed.
class ArgTypesChild extends TestArgTypes
{
    public function check($value): bool
    {
        return true;
    }

    public function number(int|float|string $value): int
    {
        return (int) $value;
    }
}

$child = new ArgTypesChild();
assert($child->check('anything'));
assert($child->number('7') === 7);

$method = new ReflectionMethod(TestArgTypes::class, 'check');
assert(!$method->getParameters()[0]->hasType());
