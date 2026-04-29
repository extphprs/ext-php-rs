<?php

declare(strict_types=1);

// Reflection proves the `#[derive(PhpUnion)]` macro emits the correct
// `zend_type` metadata via the `php_type()` override on IntoZval/FromZval.
// Internal-function arg type-checking is debug-build only on release PHP, so
// the call-site assertions below verify behaviour through actual round-trips
// of the FromZval/IntoZval impls.

$rf = new ReflectionFunction('test_php_union_param');
$params = $rf->getParameters();
assert(count($params) === 1, 'param: expected one parameter');

$type = $params[0]->getType();
assert(
    $type instanceof ReflectionUnionType,
    'param: expected ReflectionUnionType, got ' . ($type ? $type::class : 'null'),
);
assert(
    $params[0]->allowsNull() === false,
    'param: must not be nullable',
);

$members = array_map(
    static fn(ReflectionNamedType $t): string => $t->getName(),
    $type->getTypes(),
);
sort($members);
assert(
    $members === ['int', 'string'],
    'param: expected int|string, got ' . implode('|', $members),
);

$ret = $rf->getReturnType();
assert(
    $ret instanceof ReflectionNamedType,
    'param: expected ReflectionNamedType return (i64), got '
        . ($ret ? $ret::class : 'null'),
);
assert(
    $ret->getName() === 'int',
    'param: expected int return, got ' . $ret->getName(),
);

$rf = new ReflectionFunction('test_php_union_return');
$params = $rf->getParameters();
assert(count($params) === 1, 'return: expected one parameter');
assert(
    $params[0]->getType() instanceof ReflectionNamedType,
    'return: expected ReflectionNamedType (bool) param',
);
assert(
    $params[0]->getType()->getName() === 'bool',
    'return: expected bool param, got ' . $params[0]->getType()->getName(),
);

$ret = $rf->getReturnType();
assert(
    $ret instanceof ReflectionUnionType,
    'return: expected ReflectionUnionType, got ' . ($ret ? $ret::class : 'null'),
);
assert(
    $ret->allowsNull() === false,
    'return: must not be nullable',
);

$members = array_map(
    static fn(ReflectionNamedType $t): string => $t->getName(),
    $ret->getTypes(),
);
sort($members);
assert(
    $members === ['int', 'string'],
    'return: expected int|string return, got ' . implode('|', $members),
);

// End-to-end: param dispatch picks the right variant in the FromZval impl.
assert(
    test_php_union_param(42) === 1,
    'call: int input must dispatch to IntOrString::Int',
);
assert(
    test_php_union_param('hi') === 2,
    'call: string input must dispatch to IntOrString::Str',
);

// End-to-end: return dispatch picks the right variant in the IntoZval impl.
assert(
    test_php_union_return(true) === 7,
    'call: true must return the i64 variant carrying 7',
);
assert(
    test_php_union_return(false) === 'hi',
    'call: false must return the String variant carrying "hi"',
);

// `#[php_impl]` method coverage: the same machinery wires through to methods.
$rm = new ReflectionMethod('PhpUnionHolder', 'accept');
$params = $rm->getParameters();
assert(count($params) === 1, 'method: expected one parameter');

$type = $params[0]->getType();
assert(
    $type instanceof ReflectionUnionType,
    'method: expected ReflectionUnionType param, got '
        . ($type ? $type::class : 'null'),
);
$members = array_map(
    static fn(ReflectionNamedType $t): string => $t->getName(),
    $type->getTypes(),
);
sort($members);
assert(
    $members === ['int', 'string'],
    'method: expected int|string param, got ' . implode('|', $members),
);

$ret = $rm->getReturnType();
assert(
    $ret instanceof ReflectionUnionType,
    'method: expected ReflectionUnionType return, got '
        . ($ret ? $ret::class : 'null'),
);
$members = array_map(
    static fn(ReflectionNamedType $t): string => $t->getName(),
    $ret->getTypes(),
);
sort($members);
assert(
    $members === ['int', 'string'],
    'method: expected int|string return, got ' . implode('|', $members),
);

$holder = new PhpUnionHolder();
assert(
    $holder->accept(99) === 99,
    'method call: int must round-trip',
);
assert(
    $holder->accept('hello') === 'hello',
    'method call: string must round-trip',
);
