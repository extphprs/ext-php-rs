<?php

declare(strict_types = 1);

// Reflection proves we wrote the correct `zend_type` metadata for class
// unions. PHP only enforces internal-function arg types in debug builds
// (Zend/zend_execute.c: `zend_internal_call_should_throw` is `#if
// ZEND_DEBUG`), so runtime call-site enforcement is not a stable test
// surface; the assertions below mirror the slice 1-3 metadata-first style.

foreach ([
    ['fname' => 'test_class_union_arg', 'nullable' => false],
    ['fname' => 'test_class_union_nullable_arg', 'nullable' => true]
] as $case) {
    $rf = new ReflectionFunction($case['fname']);
    $params = $rf->getParameters();
    assert(count($params) === 1, "{$case['fname']}: expected one parameter");

    $type = $params[0]->getType();
    assert($type instanceof ReflectionUnionType, "{$case['fname']}: expected ReflectionUnionType");
    assert($params[0]->allowsNull() === $case['nullable'], "{$case['fname']}: nullable mismatch");

    $members = array_map(static fn(ReflectionNamedType $t): string => $t->getName(), $type->getTypes());
    sort($members);

    $expected = ['ClassUnionLeft', 'ClassUnionRight'];
    if ($case['nullable']) {
        $expected[] = 'null';
        sort($expected);
    }
    assert(
        $members === $expected,
        "{$case['fname']}: expected " . implode('|', $expected) . ', got ' . implode('|', $members)
    );
}

foreach ([
    ['fname' => 'test_class_union_returns', 'nullable' => false],
    ['fname' => 'test_class_union_nullable_returns', 'nullable' => true]
] as $case) {
    $rf = new ReflectionFunction($case['fname']);
    $ret = $rf->getReturnType();
    assert($ret instanceof ReflectionUnionType, "{$case['fname']}: expected ReflectionUnionType return");
    assert($ret->allowsNull() === $case['nullable'], "{$case['fname']}: nullable mismatch on return");

    $members = array_map(static fn(ReflectionNamedType $t): string => $t->getName(), $ret->getTypes());
    sort($members);

    $expected = ['ClassUnionLeft', 'ClassUnionRight'];
    if ($case['nullable']) {
        $expected[] = 'null';
        sort($expected);
    }
    assert(
        $members === $expected,
        "{$case['fname']}: expected " . implode('|', $expected) . ', got ' . implode('|', $members)
    );
}
