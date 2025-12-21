<?php

require(__DIR__ . '/../_utils.php');

// Tests constructor
$class = test_class('lorem ipsum', 2022);
assert($class instanceof TestClass);

// Tests getter/setter as properties (get_string -> $class->string property)
assert($class->string === 'lorem ipsum');
$class->string = 'dolor et';
assert($class->string === 'dolor et');
$class->selfRef("foo");
assert($class->string === 'Changed to foo');
$class->selfMultiRef("bar");
assert($class->string === 'Changed to bar');

assert($class->number === 2022);
$class->number = 2023;
assert($class->number === 2023);

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

// Test issue #325 - returning &'static str from getter
$staticStrClass = new TestClassStaticStrGetter();
assert($staticStrClass->static_value === 'Hello from static str');
