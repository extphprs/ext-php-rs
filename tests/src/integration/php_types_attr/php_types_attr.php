<?php

declare(strict_types=1);

// Reflection proves the macro-driven override emits the correct
// `zend_type` metadata. PHP only enforces internal-function arg types in
// debug builds, so we mirror the slice 1-3 metadata-first style.

$rf = new ReflectionFunction('test_attr_int_or_string');
$params = $rf->getParameters();
assert(count($params) === 1, 'expected one parameter');

$type = $params[0]->getType();
assert(
    $type instanceof ReflectionUnionType,
    'expected ReflectionUnionType, got ' . ($type ? $type::class : 'null'),
);
assert(
    $params[0]->allowsNull() === false,
    'must not be nullable',
);

$members = array_map(
    static fn(ReflectionNamedType $t): string => $t->getName(),
    $type->getTypes(),
);
sort($members);
assert(
    $members === ['int', 'string'],
    'expected int|string, got ' . implode('|', $members),
);

$rf = new ReflectionFunction('test_attr_returns_int_string_or_null');
$ret = $rf->getReturnType();
assert(
    $ret instanceof ReflectionUnionType,
    'expected ReflectionUnionType return, got ' . ($ret ? $ret::class : 'null'),
);
assert(
    $ret->allowsNull() === true,
    'returns_int_string_or_null must allow null',
);

$members = array_map(
    static fn(ReflectionNamedType $t): string => $t->getName(),
    $ret->getTypes(),
);
sort($members);
assert(
    $members === ['int', 'null', 'string'],
    'expected int|string|null on return, got ' . implode('|', $members),
);

$rf = new ReflectionFunction('test_attr_class_union');
$params = $rf->getParameters();
assert(count($params) === 1, 'class union: expected one parameter');

$type = $params[0]->getType();
assert(
    $type instanceof ReflectionUnionType,
    'class union: expected ReflectionUnionType, got ' . ($type ? $type::class : 'null'),
);
assert(
    $params[0]->allowsNull() === false,
    'class union must not be nullable',
);

$members = array_map(
    static fn(ReflectionNamedType $t): string => $t->getName(),
    $type->getTypes(),
);
sort($members);
assert(
    $members === ['PhpTypesAttrBar', 'PhpTypesAttrFoo'],
    'expected PhpTypesAttrFoo|PhpTypesAttrBar, got ' . implode('|', $members),
);

if (PHP_VERSION_ID >= 80300) {
    $rf = new ReflectionFunction('test_attr_intersection');
    $params = $rf->getParameters();
    assert(count($params) === 1, 'intersection: expected one parameter');

    $type = $params[0]->getType();
    assert(
        $type instanceof ReflectionIntersectionType,
        'intersection: expected ReflectionIntersectionType, got '
            . ($type ? $type::class : 'null'),
    );

    $members = array_map(
        static fn(ReflectionNamedType $t): string => $t->getName(),
        $type->getTypes(),
    );
    sort($members);
    assert(
        $members === ['Countable', 'Traversable'],
        'expected Countable&Traversable, got ' . implode('&', $members),
    );

    $rf = new ReflectionFunction('test_attr_dnf');
    $params = $rf->getParameters();
    assert(count($params) === 1, 'dnf: expected one parameter');

    $type = $params[0]->getType();
    assert(
        $type instanceof ReflectionUnionType,
        'dnf: expected ReflectionUnionType (DNF), got ' . ($type ? $type::class : 'null'),
    );

    $branches = $type->getTypes();
    assert(count($branches) === 2, 'dnf: expected two top-level branches');

    $named = [];
    $intersection = null;
    foreach ($branches as $branch) {
        if ($branch instanceof ReflectionIntersectionType) {
            assert($intersection === null, 'dnf: more than one intersection branch');
            $intersection = $branch;
            continue;
        }
        assert(
            $branch instanceof ReflectionNamedType,
            'dnf: unexpected branch class ' . $branch::class,
        );
        $named[] = $branch->getName();
    }
    sort($named);
    assert(
        $named === ['PhpTypesAttrFoo'],
        'dnf: expected named branch PhpTypesAttrFoo, got ' . implode(',', $named),
    );

    assert($intersection !== null, 'dnf: missing intersection branch');
    $intersection_members = array_map(
        static fn(ReflectionNamedType $t): string => $t->getName(),
        $intersection->getTypes(),
    );
    sort($intersection_members);
    assert(
        $intersection_members === ['Countable', 'Traversable'],
        'dnf: expected Countable&Traversable inner intersection, got '
            . implode('&', $intersection_members),
    );
}

// `#[php_impl]` method coverage: per-arg `types` and method-level `returns`.
$rm = new ReflectionMethod('PhpTypesAttrHolder', 'accept');
$params = $rm->getParameters();
assert(count($params) === 1, 'PhpTypesAttrHolder::accept: expected one parameter');

$type = $params[0]->getType();
assert(
    $type instanceof ReflectionUnionType,
    'PhpTypesAttrHolder::accept: expected ReflectionUnionType, got '
        . ($type ? $type::class : 'null'),
);

$members = array_map(
    static fn(ReflectionNamedType $t): string => $t->getName(),
    $type->getTypes(),
);
sort($members);
assert(
    $members === ['int', 'string'],
    'PhpTypesAttrHolder::accept: expected int|string, got ' . implode('|', $members),
);

$rm = new ReflectionMethod('PhpTypesAttrHolder', 'produce');
$ret = $rm->getReturnType();
assert(
    $ret instanceof ReflectionUnionType,
    'PhpTypesAttrHolder::produce: expected ReflectionUnionType, got '
        . ($ret ? $ret::class : 'null'),
);
assert(
    $ret->allowsNull() === true,
    'PhpTypesAttrHolder::produce: must allow null on return',
);

$members = array_map(
    static fn(ReflectionNamedType $t): string => $t->getName(),
    $ret->getTypes(),
);
sort($members);
assert(
    $members === ['int', 'null', 'string'],
    'PhpTypesAttrHolder::produce: expected int|string|null, got ' . implode('|', $members),
);
