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

// Test method returning Self (new instance)
$newClass = $class->withString('new string');
assert($newClass instanceof TestClass, 'withString should return TestClass instance');
assert($newClass->string === 'new string', 'new instance should have new string');
assert($class->string === 'Changed to bar', 'original instance should be unchanged');
assert($newClass !== $class, 'should be different instances');

// Test number getter/setter as property (from #[php(getter)]/[php(setter)])
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

// Test readonly class (PHP 8.2+)
if (PHP_VERSION_ID >= 80200) {
    $readonlyObj = new TestReadonlyClass('hello', 42);
    assert($readonlyObj->getName() === 'hello', 'Readonly class getter should work');
    assert($readonlyObj->getValue() === 42, 'Readonly class getter should work');

    // Check if class is marked as readonly
    $readonlyReflection = new ReflectionClass(TestReadonlyClass::class);
    assert($readonlyReflection->isReadOnly(), 'TestReadonlyClass should be marked as readonly on PHP 8.2+');
}

// Test property visibility (Issue #375)
$visibilityObj = new TestPropertyVisibility(42, 'private_data', 'protected_data');

// Test public property - should work
assert($visibilityObj->publicNum === 42, 'Public property read should work');
$visibilityObj->publicNum = 100;
assert($visibilityObj->publicNum === 100, 'Public property write should work');

// Test accessing private property through class methods - should work
assert($visibilityObj->getPrivate() === 'private_data', 'Private property access via method should work');
$visibilityObj->setPrivate('new_private');
assert($visibilityObj->getPrivate() === 'new_private', 'Private property set via method should work');

// Test accessing protected property through class methods - should work
assert($visibilityObj->getProtected() === 'protected_data', 'Protected property access via method should work');
$visibilityObj->setProtected('new_protected');
assert($visibilityObj->getProtected() === 'new_protected', 'Protected property set via method should work');

// Test that direct access to private property throws an error
assert_exception_thrown(fn() => $visibilityObj->privateStr, 'Reading private property should throw');
assert_exception_thrown(fn() => $visibilityObj->privateStr = 'test', 'Writing private property should throw');

// Test that direct access to protected property throws an error
assert_exception_thrown(fn() => $visibilityObj->protectedStr, 'Reading protected property should throw');
assert_exception_thrown(fn() => $visibilityObj->protectedStr = 'test', 'Writing protected property should throw');

// Test var_dump shows mangled names for private/protected properties
ob_start();
var_dump($visibilityObj);
$output = ob_get_clean();
assert(strpos($output, 'publicNum') !== false, 'var_dump should show public property');
// Private properties should show as ClassName::propertyName in var_dump
// Protected properties should show with * prefix

// Test reserved keyword method names
$keywordObj = new TestReservedKeywordMethods();

$result = $keywordObj->new('test value');
assert($result === 'new called with: test value', 'Method named "new" should work');

$result = $keywordObj->default();
assert($result === 'default value', 'Method named "default" should work');

$result = $keywordObj->class();
assert($result === 'TestReservedKeywordMethods', 'Method named "class" should work');

$result = $keywordObj->match('test');
assert($result === true, 'Method named "match" should work');
$result = $keywordObj->match('notfound');
assert($result === false, 'Method named "match" should work with non-matching pattern');

$result = $keywordObj->return();
assert($result === 'test value', 'Method named "return" should work');

$result = $keywordObj->static();
assert($result === 'not actually static', 'Method named "static" should work');

$reflection = new ReflectionClass(TestReservedKeywordMethods::class);
$methodNames = array_map(fn($m) => $m->getName(), $reflection->getMethods());
assert(in_array('new', $methodNames), 'Method "new" should exist in reflection');
assert(in_array('default', $methodNames), 'Method "default" should exist in reflection');
assert(in_array('class', $methodNames), 'Method "class" should exist in reflection');
assert(in_array('match', $methodNames), 'Method "match" should exist in reflection');
assert(in_array('return', $methodNames), 'Method "return" should exist in reflection');
assert(in_array('static', $methodNames), 'Method "static" should exist in reflection');

// Test final methods
$finalObj = new TestFinalMethods();
assert($finalObj->finalMethod() === 'final method result', 'Final method should work');
assert(TestFinalMethods::finalStaticMethod() === 'final static method result', 'Final static method should work');
assert($finalObj->normalMethod() === 'normal method result', 'Normal method should work');

// Verify final methods are marked as final in reflection
$finalReflection = new ReflectionClass(TestFinalMethods::class);
assert($finalReflection->getMethod('finalMethod')->isFinal(), 'finalMethod should be marked as final');
assert($finalReflection->getMethod('finalStaticMethod')->isFinal(), 'finalStaticMethod should be marked as final');
assert($finalReflection->getMethod('finalStaticMethod')->isStatic(), 'finalStaticMethod should be static');
assert(!$finalReflection->getMethod('normalMethod')->isFinal(), 'normalMethod should NOT be marked as final');

// Test abstract class
$abstractReflection = new ReflectionClass(TestAbstractClass::class);
assert($abstractReflection->isAbstract(), 'TestAbstractClass should be marked as abstract');

// Verify abstract methods are marked as abstract in reflection
assert($abstractReflection->getMethod('abstractMethod')->isAbstract(), 'abstractMethod should be marked as abstract');
assert(!$abstractReflection->getMethod('concreteMethod')->isAbstract(), 'concreteMethod should NOT be marked as abstract');

// Test extending the abstract class in PHP
class ConcreteTestClass extends TestAbstractClass {
    public function __construct() {
        parent::__construct();
    }

    public function abstractMethod(): string {
        return 'implemented abstract method';
    }
}

$concreteObj = new ConcreteTestClass();
assert($concreteObj->abstractMethod() === 'implemented abstract method', 'Implemented abstract method should work');
assert($concreteObj->concreteMethod() === 'concrete method in abstract class', 'Concrete method from abstract class should work');

// Test lazy objects (PHP 8.4+)
if (PHP_VERSION_ID >= 80400) {
    // Test with a regular (non-lazy) Rust object first
    $regularObj = new TestLazyClass('regular');
    assert(test_is_lazy($regularObj) === false, 'Regular Rust object should not be lazy');
    assert(test_is_lazy_ghost($regularObj) === false, 'Regular Rust object should not be lazy ghost');
    assert(test_is_lazy_proxy($regularObj) === false, 'Regular Rust object should not be lazy proxy');
    assert(test_is_lazy_initialized($regularObj) === false, 'Regular Rust object lazy_initialized should be false');
    // PHP 8.4 lazy objects only work with user-defined PHP classes, not internal classes.
    // Rust-defined classes are internal classes, so we test with a pure PHP class.
    class PhpLazyTestClass {
        public string $data = '';
        public bool $initialized = false;

        public function __construct(string $data) {
            $this->data = $data;
            $this->initialized = true;
        }
    }

    // Create a lazy ghost using PHP's Reflection API
    $reflector = new ReflectionClass(PhpLazyTestClass::class);
    $lazyGhost = $reflector->newLazyGhost(function (PhpLazyTestClass $obj) {
        $obj->__construct('lazy initialized');
    });

    // Verify lazy ghost introspection BEFORE initialization
    assert(test_is_lazy($lazyGhost) === true, 'Lazy ghost should be lazy');
    assert(test_is_lazy_ghost($lazyGhost) === true, 'Lazy ghost should be identified as ghost');
    assert(test_is_lazy_proxy($lazyGhost) === false, 'Lazy ghost should not be identified as proxy');
    assert(test_is_lazy_initialized($lazyGhost) === false, 'Lazy ghost should not be initialized yet');

    // Access a property to trigger initialization
    $data = $lazyGhost->data;
    assert($data === 'lazy initialized', 'Lazy ghost should be initialized with correct data');

    // Verify lazy ghost introspection AFTER initialization
    // Note: Initialized lazy ghosts become indistinguishable from regular objects (flags cleared)
    assert(test_is_lazy($lazyGhost) === false, 'Initialized lazy ghost should no longer report as lazy');
    assert(test_is_lazy_initialized($lazyGhost) === false, 'Initialized ghost returns false (not lazy anymore)');

    // Create a lazy proxy
    $lazyProxy = $reflector->newLazyProxy(function (PhpLazyTestClass $obj) {
        return new PhpLazyTestClass('proxy target');
    });

    // Verify lazy proxy introspection BEFORE initialization
    assert(test_is_lazy($lazyProxy) === true, 'Lazy proxy should be lazy');
    assert(test_is_lazy_ghost($lazyProxy) === false, 'Lazy proxy should not be identified as ghost');
    assert(test_is_lazy_proxy($lazyProxy) === true, 'Lazy proxy should be identified as proxy');
    assert(test_is_lazy_initialized($lazyProxy) === false, 'Lazy proxy should not be initialized yet');

    // Trigger initialization
    $proxyData = $lazyProxy->data;
    assert($proxyData === 'proxy target', 'Lazy proxy should forward to real instance');

    // Verify lazy proxy introspection AFTER initialization
    // Note: Initialized lazy proxies still report as lazy (IS_OBJ_LAZY_PROXY stays set)
    assert(test_is_lazy($lazyProxy) === true, 'Initialized lazy proxy should still report as lazy');
    assert(test_is_lazy_proxy($lazyProxy) === true, 'Initialized proxy should still be identified as proxy');
    assert(test_is_lazy_initialized($lazyProxy) === true, 'Lazy proxy should be initialized after property access');
}

// Test issue #325 - returning &'static str from getter
$staticStrClass = new TestClassStaticStrGetter();
assert($staticStrClass->static_value === 'Hello from static str');

// Test issue #173 - simple type syntax for extends
$baseObj = new TestBaseClass();
assert($baseObj->getBaseInfo() === 'I am the base class', 'Base class method should work');

$childObj = new TestChildClass();
assert($childObj->getChildInfo() === 'I am the child class', 'Child class method should work');

// With the trait-based workaround, inherited methods work on child instances
assert($childObj->getBaseInfo() === 'I am the base class', 'Child should have base class method via trait');

// Verify inheritance through reflection
$childReflection = new ReflectionClass(TestChildClass::class);
assert($childReflection->getParentClass()->getName() === TestBaseClass::class, 'TestChildClass should extend TestBaseClass');
assert($childObj instanceof TestBaseClass, 'TestChildClass instance should be instanceof TestBaseClass');
