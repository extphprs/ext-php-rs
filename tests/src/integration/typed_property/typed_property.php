<?php

declare(strict_types=1);

require(__DIR__ . '/../_utils.php');

// Reflection proves the runtime `zend_type` is wired correctly via
// `zend_declare_typed_property`. Slice 09 promotes property types from
// stub-only documentation to runtime-enforced types.

$rc = new ReflectionClass(TypedPropClass::class);

// 0. Property name lifecycle: every declared property must round-trip its
// name through ReflectionProperty::getName(). This guards the
// `zend_string_release` we issue immediately after `zend_declare_typed_property`
// (mirroring php-src `gen_stub.php`) — if the engine stored our caller-side
// pointer instead of copying + interning it, names would be use-after-free
// and either lookup would fail or `getName()` would return garbage.
$declaredNames = [
    'intProp', 'nullableIntProp', 'stringOrIntProp', 'fooProp', 'fooOrBarProp',
];
if (PHP_VERSION_ID >= 80100) {
    $declaredNames[] = 'intersectProp';
}
if (PHP_VERSION_ID >= 80300) {
    $declaredNames[] = 'dnfProp';
}
foreach ($declaredNames as $declaredName) {
    $reflProp = $rc->getProperty($declaredName);
    assert(
        $reflProp->getName() === $declaredName,
        "property name round-trip failed for '$declaredName', got '"
            . $reflProp->getName() . "'",
    );
}

// 1. Simple primitive (int)
$intProp = $rc->getProperty('intProp');
$intType = $intProp->getType();
assert($intType instanceof ReflectionNamedType, 'intProp: expected ReflectionNamedType');
assert($intType->getName() === 'int', 'intProp: expected int, got ' . $intType->getName());
assert(!$intType->allowsNull(), 'intProp: expected not nullable');

// 2. Nullable primitive (?int)
$nullableIntProp = $rc->getProperty('nullableIntProp');
$nullableIntType = $nullableIntProp->getType();
assert(
    $nullableIntType instanceof ReflectionNamedType,
    'nullableIntProp: expected ReflectionNamedType',
);
assert(
    $nullableIntType->getName() === 'int',
    'nullableIntProp: expected int, got ' . $nullableIntType->getName(),
);
assert($nullableIntType->allowsNull(), 'nullableIntProp: expected nullable');

// 3. Primitive union (int|string)
$unionProp = $rc->getProperty('stringOrIntProp');
$unionType = $unionProp->getType();
assert($unionType instanceof ReflectionUnionType, 'stringOrIntProp: expected ReflectionUnionType');
$members = array_map(static fn(ReflectionNamedType $t): string => $t->getName(), $unionType->getTypes());
sort($members);
assert(
    $members === ['int', 'string'],
    'stringOrIntProp: expected int|string, got ' . implode('|', $members),
);

// 4. Single class
$fooProp = $rc->getProperty('fooProp');
$fooType = $fooProp->getType();
assert($fooType instanceof ReflectionNamedType, 'fooProp: expected ReflectionNamedType');
assert(
    $fooType->getName() === 'TypedPropFooClass',
    'fooProp: expected TypedPropFooClass, got ' . $fooType->getName(),
);
assert(!$fooType->allowsNull(), 'fooProp: expected not nullable');

// 5. Class union (Foo|Bar)
$fooOrBarProp = $rc->getProperty('fooOrBarProp');
$fooOrBarType = $fooOrBarProp->getType();
assert(
    $fooOrBarType instanceof ReflectionUnionType,
    'fooOrBarProp: expected ReflectionUnionType',
);
$classMembers = array_map(
    static fn(ReflectionNamedType $t): string => $t->getName(),
    $fooOrBarType->getTypes(),
);
sort($classMembers);
assert(
    $classMembers === ['TypedPropBarClass', 'TypedPropFooClass'],
    'fooOrBarProp: expected TypedPropFooClass|TypedPropBarClass, got '
        . implode('|', $classMembers),
);

// 6. Intersection (Countable&Traversable) on PHP 8.1+
if (PHP_VERSION_ID >= 80100) {
    $intersectProp = $rc->getProperty('intersectProp');
    $intersectType = $intersectProp->getType();
    assert(
        $intersectType instanceof ReflectionIntersectionType,
        'intersectProp: expected ReflectionIntersectionType',
    );
    $intersectMembers = array_map(
        static fn(ReflectionNamedType $t): string => $t->getName(),
        $intersectType->getTypes(),
    );
    sort($intersectMembers);
    assert(
        $intersectMembers === ['Countable', 'Traversable'],
        'intersectProp: expected Countable&Traversable, got '
            . implode('&', $intersectMembers),
    );
}

// 7. DNF ((Countable&Traversable)|TypedPropFooClass) on PHP 8.3+
if (PHP_VERSION_ID >= 80300) {
    $dnfProp = $rc->getProperty('dnfProp');
    $dnfType = $dnfProp->getType();
    assert(
        $dnfType instanceof ReflectionUnionType,
        'dnfProp: expected ReflectionUnionType (DNF outer is union)',
    );
    $dnfTypeStrings = array_map(
        static fn($t): string => $t instanceof ReflectionIntersectionType
            ? '(' . implode('&', array_map(
                static fn(ReflectionNamedType $n): string => $n->getName(),
                $t->getTypes(),
            )) . ')'
            : $t->getName(),
        $dnfType->getTypes(),
    );
    sort($dnfTypeStrings);
    assert(
        $dnfTypeStrings === ['(Countable&Traversable)', 'TypedPropFooClass'],
        'dnfProp: expected (Countable&Traversable)|TypedPropFooClass, got '
            . implode('|', $dnfTypeStrings),
    );
}

// Runtime enforcement: TypeError on bad assignments
$obj = new TypedPropClass();

// intProp must reject string
$caught = false;
try {
    $obj->intProp = 'not an int';
} catch (TypeError) {
    $caught = true;
}
assert($caught, 'intProp must reject string assignment with TypeError');

// nullableIntProp accepts null
$obj->nullableIntProp = null;
assert($obj->nullableIntProp === null, 'nullableIntProp must accept null');
$obj->nullableIntProp = 42;
assert($obj->nullableIntProp === 42, 'nullableIntProp must accept int');

// stringOrIntProp accepts string and int but rejects array
$obj->stringOrIntProp = 'hello';
assert($obj->stringOrIntProp === 'hello', 'stringOrIntProp must accept string');
$obj->stringOrIntProp = 7;
assert($obj->stringOrIntProp === 7, 'stringOrIntProp must accept int');
$caught = false;
try {
    $obj->stringOrIntProp = [];
} catch (TypeError) {
    $caught = true;
}
assert($caught, 'stringOrIntProp must reject array assignment');

// fooProp accepts TypedPropFooClass but rejects TypedPropBarClass
$obj->fooProp = new TypedPropFooClass();
$caught = false;
try {
    $obj->fooProp = new TypedPropBarClass();
} catch (TypeError) {
    $caught = true;
}
assert($caught, 'fooProp must reject TypedPropBarClass assignment');

// fooOrBarProp accepts both
$obj->fooOrBarProp = new TypedPropFooClass();
$obj->fooOrBarProp = new TypedPropBarClass();
$caught = false;
try {
    $obj->fooOrBarProp = new stdClass();
} catch (TypeError) {
    $caught = true;
}
assert($caught, 'fooOrBarProp must reject stdClass assignment');

if (PHP_VERSION_ID >= 80100) {
    // intersectProp accepts ArrayObject (Countable+Traversable) but rejects stdClass
    $obj->intersectProp = new ArrayObject();
    $caught = false;
    try {
        $obj->intersectProp = new stdClass();
    } catch (TypeError) {
        $caught = true;
    }
    assert($caught, 'intersectProp must reject stdClass assignment');
}

if (PHP_VERSION_ID >= 80300) {
    // dnfProp accepts ArrayObject (matches first arm) and TypedPropFooClass (matches second)
    $obj->dnfProp = new ArrayObject();
    $obj->dnfProp = new TypedPropFooClass();
    $caught = false;
    try {
        $obj->dnfProp = new TypedPropBarClass();
    } catch (TypeError) {
        $caught = true;
    }
    assert($caught, 'dnfProp must reject TypedPropBarClass assignment');
}

// IS_UNDEF default: a typed property registered with no explicit default must
// be in `IS_PROP_UNINIT` state, so reading before the first assignment throws
// `Error`. Guards the `Zval::undef()` path in `register_property` — if we
// used `Zval::new()` (`IS_NULL`), declaration would either fail with
// TypeError on non-nullable properties or the read would silently return
// null on nullable ones.
$freshObj = new TypedPropClass();
$thrown = null;
try {
    $_ = $freshObj->intProp;
} catch (Error $e) {
    $thrown = $e;
}
assert(
    $thrown instanceof Error,
    'reading uninitialized typed property must throw Error '
        . '(IS_PROP_UNINIT semantics)',
);

echo "typed_property: ok\n";
