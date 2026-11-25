<?php

// A class with no constructor whose Rust value has a default declares none,
// as a PHP class without a constructor does.
$reflection = new ReflectionClass(TestNoConstructor::class);
assert($reflection->getConstructor() === null, 'no constructor is declared');
assert($reflection->isInstantiable());

$object = new TestNoConstructor();
assert($object->increment() === 1);
assert($object->increment() === 2);

// A PHP subclass is an object of the subclass, initialized from `Default`,
// with no constructor of its own either.
class NoConstructorChild extends TestNoConstructor
{
    public string $extra = 'child';
}

$child = new NoConstructorChild();
assert($child::class === NoConstructorChild::class, $child::class);
assert($child instanceof TestNoConstructor);
assert($child->extra === 'child');
assert($child->increment() === 1);
assert(( new ReflectionClass($child) )->getConstructor() === null);

// A subclass constructor cannot call a parent constructor that does not exist,
// exactly as with a PHP parent class without one.
class NoConstructorChildCallingParent extends TestNoConstructor
{
    public function __construct()
    {
        parent::__construct();
    }
}

try {
    new NoConstructorChildCallingParent();
    assert(false, 'parent::__construct() must fail');
} catch (Error $e) {
    assert(str_contains($e->getMessage(), 'Cannot call constructor'), $e->getMessage());
}

// Clones keep the subclass.
$copy = clone $child;
assert($copy::class === NoConstructorChild::class);

// An abstract class: a PHP subclass implementing the abstract methods needs no
// constructor.
class ConcreteNoConstructor extends TestNoConstructorAbstract
{
    public function name(): string
    {
        return 'concrete';
    }
}

$concrete = new ConcreteNoConstructor();
assert($concrete->name() === 'concrete');
assert($concrete->greet() === 'hello from the abstract class');
assert(( new ReflectionClass(TestNoConstructorAbstract::class) )->getConstructor() === null);

try {
    new TestNoConstructorAbstract();
    assert(false, 'an abstract class cannot be instantiated');
} catch (Error $e) {
    assert(str_contains($e->getMessage(), 'Cannot instantiate abstract class'), $e->getMessage());
}

// Without `#[php(no_constructor)]`, a class with a default value keeps the
// constructor that throws.
assert(( new ReflectionClass(TestNoConstructorNotOptedIn::class) )->getConstructor() !== null);
try {
    new TestNoConstructorNotOptedIn();
    assert(false, 'a class that did not opt in cannot be instantiated');
} catch (Exception $e) {
    assert($e->getMessage() === 'You cannot instantiate this class from PHP.', $e->getMessage());
}

// Under a Rust class with a constructor, the class keeps the constructor that
// throws: inheriting the parent's would run it on an object of another type.
$runs = TestConstructingBase::runs();
new TestConstructingBase();
assert(TestConstructingBase::runs() === ( $runs + 1 ));
try {
    new TestNoConstructorChild();
    assert(false, 'the Rust parent constructor must not be inherited');
} catch (Exception $e) {
    assert($e->getMessage() === 'You cannot instantiate this class from PHP.', $e->getMessage());
}
assert(TestConstructingBase::runs() === ( $runs + 1 ), 'the parent constructor did not run');
