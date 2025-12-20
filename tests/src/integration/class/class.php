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
assert($classReflection->getMethod('private')->isPrivate());
assert($classReflection->getMethod('protected')->isProtected());

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
