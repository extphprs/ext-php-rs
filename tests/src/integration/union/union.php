<?php

require(__DIR__ . '/../_utils.php');

// Test union types via reflection (PHP 8.0+)

// Helper to get type string from ReflectionType
function getTypeString(ReflectionType|null $type): string {
    if ($type === null) {
        return 'mixed';
    }

    if ($type instanceof ReflectionUnionType) {
        $types = array_map(fn($t) => $t->getName(), $type->getTypes());
        sort($types); // Sort for consistent comparison
        return implode('|', $types);
    }

    if ($type instanceof ReflectionNamedType) {
        $name = $type->getName();
        if ($type->allowsNull() && $name !== 'mixed' && $name !== 'null') {
            return '?' . $name;
        }
        return $name;
    }

    return (string)$type;
}

// Test int|string union type
$func = new ReflectionFunction('test_union_int_string');
$params = $func->getParameters();
assert(count($params) === 1, 'test_union_int_string should have 1 parameter');

$paramType = $params[0]->getType();
assert($paramType instanceof ReflectionUnionType, 'Parameter should be a union type');

$typeStr = getTypeString($paramType);
assert($typeStr === 'int|string', "Expected 'int|string', got '$typeStr'");

// Call the function with int
$result = test_union_int_string(42);
assert($result === 'ok', 'Function should accept int');

// Call the function with string
$result = test_union_int_string("hello");
assert($result === 'ok', 'Function should accept string');

// Test int|string|null union type
$func = new ReflectionFunction('test_union_int_string_null');
$params = $func->getParameters();
$paramType = $params[0]->getType();
assert($paramType instanceof ReflectionUnionType, 'Parameter should be a union type');

$typeStr = getTypeString($paramType);
assert($typeStr === 'int|null|string', "Expected 'int|null|string', got '$typeStr'");

// Call with null
$result = test_union_int_string_null(null);
assert($result === 'ok', 'Function should accept null');

// Test array|bool union type
$func = new ReflectionFunction('test_union_array_bool');
$params = $func->getParameters();
$paramType = $params[0]->getType();
assert($paramType instanceof ReflectionUnionType, 'Parameter should be a union type');

$typeStr = getTypeString($paramType);
assert($typeStr === 'array|bool', "Expected 'array|bool', got '$typeStr'");

// Call with array
$result = test_union_array_bool([1, 2, 3]);
assert($result === 'ok', 'Function should accept array');

// Call with bool
$result = test_union_array_bool(true);
assert($result === 'ok', 'Function should accept bool');

// ==== Macro-based union type tests ====

// Test macro-based int|string union type
$func = new ReflectionFunction('test_macro_union_int_string');
$params = $func->getParameters();
assert(count($params) === 1, 'test_macro_union_int_string should have 1 parameter');

$paramType = $params[0]->getType();
assert($paramType instanceof ReflectionUnionType, 'Parameter should be a union type');

$typeStr = getTypeString($paramType);
assert($typeStr === 'int|string', "Expected 'int|string', got '$typeStr'");

// Call macro function with int
$result = test_macro_union_int_string(42);
assert($result === 'macro_ok', 'Macro function should accept int');

// Call macro function with string
$result = test_macro_union_int_string("hello");
assert($result === 'macro_ok', 'Macro function should accept string');

// Test macro-based float|bool|null union type
$func = new ReflectionFunction('test_macro_union_float_bool_null');
$params = $func->getParameters();
$paramType = $params[0]->getType();
assert($paramType instanceof ReflectionUnionType, 'Parameter should be a union type');

$typeStr = getTypeString($paramType);
assert($typeStr === 'bool|float|null', "Expected 'bool|float|null', got '$typeStr'");

// Call with float
$result = test_macro_union_float_bool_null(3.14);
assert($result === 'macro_ok', 'Macro function should accept float');

// Call with bool
$result = test_macro_union_float_bool_null(false);
assert($result === 'macro_ok', 'Macro function should accept bool');

// Call with null
$result = test_macro_union_float_bool_null(null);
assert($result === 'macro_ok', 'Macro function should accept null');

// ==== Intersection type tests ====
// Note: Intersection types for internal function parameters require PHP 8.3+
// On PHP 8.1/8.2, intersection types exist for userland code but internal functions
// fall back to 'mixed' type. See: https://github.com/php/php-src/pull/11969
if (PHP_VERSION_ID >= 80100) {
    $arrayIterator = new ArrayIterator([1, 2, 3]);

    // Function calls work on PHP 8.1+ (the function itself accepts the value)
    echo "Testing intersection type function call (FunctionBuilder)...\n";
    $result = test_intersection_countable_traversable($arrayIterator);
    assert($result === 'intersection_ok', 'Function should accept ArrayIterator');
    echo "Function call succeeded!\n";

    echo "Testing macro-based intersection type function call...\n";
    $result = test_macro_intersection($arrayIterator);
    assert($result === 'macro_intersection_ok', 'Macro function should accept ArrayIterator');
    echo "Macro function call succeeded!\n";

    // Reflection tests for intersection types in internal functions require PHP 8.3+
    if (PHP_VERSION_ID >= 80300) {
        // Now test intersection type via reflection
        echo "Testing intersection type via reflection (FunctionBuilder)...\n";
        $func = new ReflectionFunction('test_intersection_countable_traversable');
        $params = $func->getParameters();
        assert(count($params) === 1, 'test_intersection_countable_traversable should have 1 parameter');

        $paramType = $params[0]->getType();
        assert($paramType instanceof ReflectionIntersectionType, 'Parameter should be an intersection type');

        $types = array_map(fn($t) => $t->getName(), $paramType->getTypes());
        sort($types);
        $typeStr = implode('&', $types);
        assert($typeStr === 'Countable&Traversable', "Expected 'Countable&Traversable', got '$typeStr'");

        // Test macro intersection type via reflection
        echo "Testing macro-based intersection type via reflection...\n";
        $func = new ReflectionFunction('test_macro_intersection');
        $params = $func->getParameters();
        assert(count($params) === 1, 'test_macro_intersection should have 1 parameter');

        $paramType = $params[0]->getType();
        assert($paramType instanceof ReflectionIntersectionType, 'Macro parameter should be an intersection type');

        $types = array_map(fn($t) => $t->getName(), $paramType->getTypes());
        sort($types);
        $typeStr = implode('&', $types);
        assert($typeStr === 'Countable&Traversable', "Expected 'Countable&Traversable' for macro, got '$typeStr'");

        echo "All intersection type reflection tests passed!\n";
    } else {
        echo "Skipping intersection type reflection tests (requires PHP 8.3+ for internal functions).\n";
    }

    echo "All intersection type function call tests passed!\n";
}

// ==== DNF type tests (PHP 8.2+) ====
// Note: DNF types for internal function parameters require PHP 8.3+
// See: https://github.com/php/php-src/pull/11969
if (PHP_VERSION_ID >= 80200) {
    $arrayIterator = new ArrayIterator([1, 2, 3]);
    $arrayObject = new ArrayObject([1, 2, 3]);

    // Function calls work on PHP 8.2+ (the function itself accepts the value)
    echo "Testing DNF type function call (FunctionBuilder)...\n";
    $result = test_dnf($arrayIterator);
    assert($result === 'dnf_ok', 'Function should accept ArrayIterator (satisfies Countable&Traversable)');
    echo "Function call with intersection part succeeded!\n";

    $result = test_dnf($arrayObject);
    assert($result === 'dnf_ok', 'Function should accept ArrayObject (implements ArrayAccess)');
    echo "Function call with ArrayAccess part succeeded!\n";

    echo "Testing macro-based DNF type function call...\n";
    $result = test_macro_dnf($arrayIterator);
    assert($result === 'macro_dnf_ok', 'Macro function should accept ArrayIterator');
    echo "Macro function call succeeded!\n";

    echo "Testing multi-intersection DNF type...\n";
    $result = test_macro_dnf_multi($arrayIterator);
    assert($result === 'macro_dnf_multi_ok', 'Multi-intersection DNF should accept ArrayIterator');
    echo "Multi-intersection DNF function call succeeded!\n";

    // Reflection tests for DNF types in internal functions require PHP 8.3+
    if (PHP_VERSION_ID >= 80300) {
        // Test DNF type via reflection
        echo "Testing DNF type via reflection (FunctionBuilder)...\n";
        $func = new ReflectionFunction('test_dnf');
        $params = $func->getParameters();
        assert(count($params) === 1, 'test_dnf should have 1 parameter');

        $paramType = $params[0]->getType();
        assert($paramType instanceof ReflectionUnionType, 'Parameter should be a union type (DNF)');

        // DNF types are unions at the top level, with intersection types as members
        $types = $paramType->getTypes();
        assert(count($types) === 2, 'DNF type should have 2 members');

        // Check that we have both an intersection type and a named type
        $hasIntersection = false;
        $hasNamed = false;
        foreach ($types as $type) {
            if ($type instanceof ReflectionIntersectionType) {
                $hasIntersection = true;
                $intersectionTypes = array_map(fn($t) => $t->getName(), $type->getTypes());
                sort($intersectionTypes);
                assert($intersectionTypes === ['Countable', 'Traversable'],
                    'Intersection should be Countable&Traversable');
            } elseif ($type instanceof ReflectionNamedType) {
                $hasNamed = true;
                assert($type->getName() === 'ArrayAccess', 'Named type should be ArrayAccess');
            }
        }
        assert($hasIntersection, 'DNF type should contain an intersection type');
        assert($hasNamed, 'DNF type should contain a named type');

        // Test macro DNF type via reflection
        echo "Testing macro-based DNF type via reflection...\n";
        $func = new ReflectionFunction('test_macro_dnf');
        $params = $func->getParameters();
        $paramType = $params[0]->getType();
        assert($paramType instanceof ReflectionUnionType, 'Macro parameter should be a union type (DNF)');

        $func = new ReflectionFunction('test_macro_dnf_multi');
        $params = $func->getParameters();
        $paramType = $params[0]->getType();
        assert($paramType instanceof ReflectionUnionType, 'Multi DNF parameter should be a union type');

        $types = $paramType->getTypes();
        assert(count($types) === 2, 'Multi DNF type should have 2 intersection members');
        foreach ($types as $type) {
            assert($type instanceof ReflectionIntersectionType, 'Each member should be an intersection type');
        }

        echo "All DNF type reflection tests passed!\n";
    } else {
        echo "Skipping DNF type reflection tests (requires PHP 8.3+ for internal functions).\n";
    }

    echo "All DNF type function call tests passed!\n";
}

// ==== PhpUnion derive macro tests ====
// Tests for the #[derive(PhpUnion)] macro which allows representing PHP union types as Rust enums

echo "Testing PhpUnion derive macro (int|string)...\n";

// Test int|string union via reflection
$func = new ReflectionFunction('test_php_union_enum');
$params = $func->getParameters();
assert(count($params) === 1, 'test_php_union_enum should have 1 parameter');

$paramType = $params[0]->getType();
assert($paramType instanceof ReflectionUnionType, 'PhpUnion parameter should be a union type');

$typeStr = getTypeString($paramType);
assert($typeStr === 'int|string', "Expected 'int|string', got '$typeStr'");

// Call with int - should match IntOrString::Int variant
$result = test_php_union_enum(42);
assert($result === 'int:42', "Expected 'int:42', got '$result'");

// Call with string - should match IntOrString::Str variant
$result = test_php_union_enum("hello");
assert($result === 'string:hello', "Expected 'string:hello', got '$result'");

echo "PhpUnion int|string tests passed!\n";

echo "Testing PhpUnion derive macro (float|bool)...\n";

// Test float|bool union via reflection
$func = new ReflectionFunction('test_php_union_float_bool');
$params = $func->getParameters();
$paramType = $params[0]->getType();
assert($paramType instanceof ReflectionUnionType, 'PhpUnion parameter should be a union type');

$typeStr = getTypeString($paramType);
assert($typeStr === 'bool|float', "Expected 'bool|float', got '$typeStr'");

// Call with float
$result = test_php_union_float_bool(3.14);
assert($result === 'float:3.14', "Expected 'float:3.14', got '$result'");

// Call with bool
$result = test_php_union_float_bool(true);
assert($result === 'bool:true', "Expected 'bool:true', got '$result'");

echo "PhpUnion float|bool tests passed!\n";

// Note: PhpUnion derive with interface/class types requires types that implement
// both FromZval and IntoZval. For object types, use the macro-based syntax
// #[php(types = "...")] instead. DNF types are tested through macro-based tests.

echo "All union type tests passed!\n";
