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

// Slice 2: nullable union, in both spellings.
foreach (
    ['test_union_int_string_or_null', 'test_union_int_string_allow_null'] as $fname
) {
    $rf = new ReflectionFunction($fname);
    $param = $rf->getParameters()[0];
    $type = $param->getType();
    assert($type instanceof ReflectionUnionType, "$fname: expected union type");
    assert($param->allowsNull() === true, "$fname: expected nullable");

    $members = array_map(
        static fn(ReflectionNamedType $t): string => $t->getName(),
        $type->getTypes(),
    );
    sort($members);
    assert(
        $members === ['int', 'null', 'string'],
        "$fname: expected int|null|string, got " . implode('|', $members),
    );

    assert($fname(42) === 1, "$fname: int call");
    assert($fname("hello") === 2, "$fname: string call");
    assert($fname(null) === 3, "$fname: null call");
}

// Slice 3: union return types.
foreach ([
    [
        'fname' => 'test_returns_int_or_string',
        'nullable' => false,
        'members' => ['int', 'string'],
    ],
    [
        'fname' => 'test_returns_int_string_or_null',
        'nullable' => true,
        'members' => ['int', 'null', 'string'],
    ],
] as $case) {
    $rf = new ReflectionFunction($case['fname']);
    $ret = $rf->getReturnType();
    assert(
        $ret instanceof ReflectionUnionType,
        "{$case['fname']}: expected ReflectionUnionType return",
    );
    assert(
        $ret->allowsNull() === $case['nullable'],
        "{$case['fname']}: nullable mismatch",
    );

    $members = array_map(
        static fn(ReflectionNamedType $t): string => $t->getName(),
        $ret->getTypes(),
    );
    sort($members);
    assert(
        $members === $case['members'],
        "{$case['fname']}: expected " . implode('|', $case['members'])
            . ", got " . implode('|', $members),
    );
}
