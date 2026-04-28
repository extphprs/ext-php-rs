<?php

declare(strict_types=1);

// Reflection proves we wrote the correct `zend_type` metadata.
// PHP itself only enforces internal-function arg types in debug builds
// (Zend/zend_execute.c: zend_internal_call_should_throw is `#if ZEND_DEBUG`),
// so runtime call-site enforcement is not a stable test surface.
$rf = new ReflectionFunction('test_union_int_or_string');
$params = $rf->getParameters();
assert(count($params) === 1, 'expected exactly one parameter');

$type = $params[0]->getType();
assert($type instanceof ReflectionUnionType, 'expected a union type');
assert($params[0]->allowsNull() === false, 'union must not be nullable');

$members = array_map(static fn(ReflectionNamedType $t): string => $t->getName(), $type->getTypes());
sort($members);
assert($members === ['int', 'string'], 'expected int|string members, got ' . implode('|', $members));

// End-to-end: function is callable with each accepted member.
assert(test_union_int_or_string(42) === 1);
assert(test_union_int_or_string("hello") === 2);
