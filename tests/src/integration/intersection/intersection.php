<?php

declare(strict_types = 1);

// Reflection proves we wrote the correct `zend_type` metadata for class
// intersections. PHP only enforces internal-function arg types in debug
// builds (Zend/zend_execute.c: `zend_internal_call_should_throw` is `#if
// ZEND_DEBUG`), so runtime call-site enforcement is not a stable test
// surface; the assertions below mirror the class_union slice's
// metadata-first style.

$rf = new ReflectionFunction('test_intersection_arg');
$params = $rf->getParameters();
assert(count($params) === 1, 'test_intersection_arg: expected one parameter');

$type = $params[0]->getType();
assert($type instanceof ReflectionIntersectionType, 'test_intersection_arg: expected ReflectionIntersectionType');
assert($params[0]->allowsNull() === false, 'test_intersection_arg: nullable intersections are deferred to slice 04');

$members = array_map(static fn(ReflectionNamedType $t): string => $t->getName(), $type->getTypes());
sort($members);
$expected = ['Countable', 'Traversable'];
assert(
    $members === $expected,
    'test_intersection_arg: expected ' . implode('&', $expected) . ', got ' . implode('&', $members)
);

$rf = new ReflectionFunction('test_intersection_returns');
$ret = $rf->getReturnType();
assert(
    $ret instanceof ReflectionIntersectionType,
    'test_intersection_returns: expected ReflectionIntersectionType return'
);
assert($ret->allowsNull() === false, 'test_intersection_returns: nullable intersections are deferred to slice 04');

$members = array_map(static fn(ReflectionNamedType $t): string => $t->getName(), $ret->getTypes());
sort($members);
assert(
    $members === $expected,
    'test_intersection_returns: expected ' . implode('&', $expected) . ', got ' . implode('&', $members)
);
