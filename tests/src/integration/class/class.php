<?php

require(__DIR__ . '/../_utils.php');

// Tests constructor
$class = test_class('lorem ipsum', 2022);
assert($class instanceof TestClass);

// Tests getter/setter
assert($class->getString() === 'lorem ipsum');
$class->setString('dolor et');
assert($class->getString() === 'dolor et');
$class->selfRef("foo");
assert($class->getString() === 'Changed to foo');
$class->selfMultiRef("bar");
assert($class->getString() === 'Changed to bar');

// Test method returning Self (new instance)
$newClass = $class->withString('new string');
assert($newClass instanceof TestClass, 'withString should return TestClass instance');
assert($newClass->getString() === 'new string', 'new instance should have new string');
assert($class->getString() === 'Changed to bar', 'original instance should be unchanged');
assert($newClass !== $class, 'should be different instances');

assert($class->getNumber() === 2022);
$class->setNumber(2023);
assert($class->getNumber() === 2023);

var_dump($class);
// Tests #prop decorator
assert($class->booleanProp);
$class->booleanProp = false;
assert($class->booleanProp === false);

// Call regular from object
assert($class->staticCall('Php') === 'Hello Php');

// Call static from object
assert($class::staticCall('Php') === 'Hello Php');

// Call static from class
assert(TestClass::staticCall('Php') === 'Hello Php');

$ex = new TestClassExtends();
assert_exception_thrown(fn() => throw $ex);
assert_exception_thrown(fn() => throwException());

$arrayAccess = new TestClassArrayAccess();
assert_exception_thrown(fn() => $arrayAccess[0] = 'foo');
assert_exception_thrown(fn() => $arrayAccess['foo']);
assert($arrayAccess[0] === true);
assert($arrayAccess[1] === false);

$classReflection = new ReflectionClass(TestClassMethodVisibility::class);
assert($classReflection->getMethod('__construct')->isPrivate());
assert($classReflection->getMethod('privateMethod')->isPrivate());
assert($classReflection->getMethod('protectedMethod')->isProtected());

$classReflection = new ReflectionClass(TestClassProtectedConstruct::class);
assert($classReflection->getMethod('__construct')->isProtected());

// Test static properties (Issue #252)
$staticObj = new TestStaticProps(42);
assert($staticObj->instanceValue === 42, 'Instance property should work');

// Verify static property exists and is accessible
$reflection = new ReflectionClass(TestStaticProps::class);
$staticCounterProp = $reflection->getProperty('staticCounter');
assert($staticCounterProp->isStatic(), 'staticCounter should be a static property');
assert($staticCounterProp->isPublic(), 'staticCounter should be public');

// Verify private static property
$privateStaticProp = $reflection->getProperty('privateStatic');
assert($privateStaticProp->isStatic(), 'privateStatic should be a static property');
assert($privateStaticProp->isPrivate(), 'privateStatic should be private');

// Test accessing static property via class
TestStaticProps::$staticCounter = 100;
assert(TestStaticProps::$staticCounter === 100, 'Should be able to set and get static property');

// Test that static property is shared across instances
$obj1 = new TestStaticProps(1);
$obj2 = new TestStaticProps(2);
TestStaticProps::$staticCounter = 200;
assert(TestStaticProps::$staticCounter === 200, 'Static property value should be shared');

// Test static methods that interact with static properties
TestStaticProps::setCounter(0);
assert(TestStaticProps::getCounter() === 0, 'Counter should be 0 after reset');

TestStaticProps::incrementCounter();
assert(TestStaticProps::getCounter() === 1, 'Counter should be 1 after increment');

TestStaticProps::incrementCounter();
TestStaticProps::incrementCounter();
assert(TestStaticProps::getCounter() === 3, 'Counter should be 3 after 3 increments');

// Test that PHP access and Rust access see the same value
TestStaticProps::$staticCounter = 50;
assert(TestStaticProps::getCounter() === 50, 'Rust should see PHP-set value');

TestStaticProps::setCounter(100);
assert(TestStaticProps::$staticCounter === 100, 'PHP should see Rust-set value');

// Test FluentBuilder - returning $this for method chaining (Issue #502)
$builder = new FluentBuilder();
assert($builder->getValue() === 0);
assert($builder->getName() === '');

// Test single method call returning $this
$result = $builder->setValue(42);
assert($result === $builder, 'setValue should return $this');
assert($builder->getValue() === 42);

// Test fluent interface / method chaining
$builder2 = new FluentBuilder();
$chainResult = $builder2->setValue(100)->setName('test');
assert($chainResult === $builder2, 'Chained methods should return $this');
assert($builder2->getValue() === 100);
assert($builder2->getName() === 'test');

// Test returning &Self (immutable reference)
$selfRef = $builder2->getSelf();
assert($selfRef === $builder2, 'getSelf should return $this');

// ==== Union types in class methods tests ====

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

// Test union type in instance method
$unionObj = new TestUnionMethods();

$method = new ReflectionMethod(TestUnionMethods::class, 'acceptIntOrString');
$params = $method->getParameters();
assert(count($params) === 1, 'acceptIntOrString should have 1 parameter');

$paramType = $params[0]->getType();
assert($paramType instanceof ReflectionUnionType, 'Parameter should be a union type');

$typeStr = getTypeString($paramType);
assert($typeStr === 'int|string', "Expected 'int|string', got '$typeStr'");

// Call method with int
$result = $unionObj->acceptIntOrString(42);
assert($result === 'method_ok', 'Method should accept int');

// Call method with string
$result = $unionObj->acceptIntOrString("hello");
assert($result === 'method_ok', 'Method should accept string');

// Test union type in static method
$method = new ReflectionMethod(TestUnionMethods::class, 'acceptFloatBoolNull');
$params = $method->getParameters();
$paramType = $params[0]->getType();
assert($paramType instanceof ReflectionUnionType, 'Static method parameter should be a union type');

$typeStr = getTypeString($paramType);
assert($typeStr === 'bool|float|null', "Expected 'bool|float|null', got '$typeStr'");

// Call static method with various types
$result = TestUnionMethods::acceptFloatBoolNull(3.14);
assert($result === 'static_method_ok', 'Static method should accept float');

$result = TestUnionMethods::acceptFloatBoolNull(true);
assert($result === 'static_method_ok', 'Static method should accept bool');

$result = TestUnionMethods::acceptFloatBoolNull(null);
assert($result === 'static_method_ok', 'Static method should accept null');

echo "All class union type tests passed!\n";
