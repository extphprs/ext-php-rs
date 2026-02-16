<?php

declare(strict_types=1);

// Test existing functionality: Interface existence and explicit extends
assert(interface_exists('ExtPhpRs\Interface\EmptyObjectInterface'), 'Interface not exist');
assert(is_a('ExtPhpRs\Interface\EmptyObjectInterface', Throwable::class, true), 'Interface could extend Throwable');


final class Test extends Exception implements ExtPhpRs\Interface\EmptyObjectInterface
{
	public static function void(): void
	{
	}

	public function nonStatic(string $data): string
	{
		return sprintf('%s - TEST', $data);
	}

	public function refToLikeThisClass(
		string $data,
		ExtPhpRs\Interface\EmptyObjectInterface $other,
	): string {
		return sprintf('%s | %s', $this->nonStatic($data), $other->nonStatic($data));
	}

    public function setValue(int $value = 0): void {

    }
}
$f = new Test();

assert(is_a($f, Throwable::class));
assert($f->nonStatic('Rust') === 'Rust - TEST');
assert($f->refToLikeThisClass('TEST', $f) === 'TEST - TEST | TEST - TEST');
assert(ExtPhpRs\Interface\EmptyObjectInterface::STRING_CONST === 'STRING_CONST');
assert(ExtPhpRs\Interface\EmptyObjectInterface::USIZE_CONST === 200);

// Test Feature 1: Interface inheritance via Rust trait bounds
assert(interface_exists('ExtPhpRs\Interface\ParentInterface'), 'ParentInterface should exist');
assert(interface_exists('ExtPhpRs\Interface\ChildInterface'), 'ChildInterface should exist');
assert(
    is_a('ExtPhpRs\Interface\ChildInterface', 'ExtPhpRs\Interface\ParentInterface', true),
    'ChildInterface should extend ParentInterface via Rust trait bounds'
);

// ============================================================================
// Test Feature 2: Implementing PHP's built-in Iterator interface (issue #308)
// This demonstrates how Rust objects can be used with PHP's foreach loop
// ============================================================================

// Test RangeIterator - a simple numeric range iterator
assert(class_exists('ExtPhpRs\Interface\RangeIterator'), 'RangeIterator class should exist');

$range = new ExtPhpRs\Interface\RangeIterator(1, 5);
assert($range instanceof Iterator, 'RangeIterator should implement Iterator interface');
assert($range instanceof Traversable, 'RangeIterator should implement Traversable interface');

// Test foreach functionality with RangeIterator
$collected = [];
foreach ($range as $key => $value) {
    $collected[$key] = $value;
}
assert($collected === [0 => 1, 1 => 2, 2 => 3, 3 => 4, 4 => 5], 'RangeIterator should iterate correctly');

// Test that we can iterate multiple times (rewind works)
$sum = 0;
foreach ($range as $value) {
    $sum += $value;
}
assert($sum === 15, 'RangeIterator should be rewindable and sum to 15');

// Test empty range
$emptyRange = new ExtPhpRs\Interface\RangeIterator(5, 1);
$emptyCollected = [];
foreach ($emptyRange as $value) {
    $emptyCollected[] = $value;
}
assert($emptyCollected === [], 'Empty range should produce no iterations');

// Test single element range
$singleRange = new ExtPhpRs\Interface\RangeIterator(42, 42);
$singleCollected = [];
foreach ($singleRange as $key => $value) {
    $singleCollected[$key] = $value;
}
assert($singleCollected === [0 => 42], 'Single element range should work');

// Test MapIterator - string keys and values
assert(class_exists('ExtPhpRs\Interface\MapIterator'), 'MapIterator class should exist');

$map = new ExtPhpRs\Interface\MapIterator();
assert($map instanceof Iterator, 'MapIterator should implement Iterator interface');

$mapCollected = [];
foreach ($map as $key => $value) {
    $mapCollected[$key] = $value;
}
assert($mapCollected === ['first' => 'one', 'second' => 'two', 'third' => 'three'],
    'MapIterator should iterate with string keys and values');

// Test VecIterator - dynamic content iterator
assert(class_exists('ExtPhpRs\Interface\VecIterator'), 'VecIterator class should exist');

$vec = new ExtPhpRs\Interface\VecIterator();
assert($vec instanceof Iterator, 'VecIterator should implement Iterator interface');

// Test empty iterator
$emptyVecCollected = [];
foreach ($vec as $value) {
    $emptyVecCollected[] = $value;
}
assert($emptyVecCollected === [], 'Empty VecIterator should produce no iterations');

// Add items and iterate (VecIterator stores i64 values)
$vec->push(100);
$vec->push(200);
$vec->push(300);

$vecCollected = [];
foreach ($vec as $key => $value) {
    $vecCollected[$key] = $value;
}
assert(count($vecCollected) === 3, 'VecIterator should have 3 items');
assert($vecCollected[0] === 100, 'First item should be 100');
assert($vecCollected[1] === 200, 'Second item should be 200');
assert($vecCollected[2] === 300, 'Third item should be 300');

// Test iterator_to_array() function works
$range2 = new ExtPhpRs\Interface\RangeIterator(10, 12);
$arr = iterator_to_array($range2);
assert($arr === [0 => 10, 1 => 11, 2 => 12], 'iterator_to_array should work with RangeIterator');

// Test iterator_count() function works
$range3 = new ExtPhpRs\Interface\RangeIterator(1, 100);
$count = iterator_count($range3);
assert($count === 100, 'iterator_count should return 100 for range 1-100');

// ============================================================================
// Test Feature 3: #[php_impl_interface] - Rust class implementing custom interface
// This demonstrates how a Rust struct can implement a PHP interface defined with
// #[php_interface] using the #[php_impl_interface] attribute
// ============================================================================

assert(class_exists('ExtPhpRs\Interface\Greeter'), 'Greeter class should exist');

$greeter = new ExtPhpRs\Interface\Greeter('World');

// Test that Greeter implements ParentInterface via #[php_impl_interface]
assert($greeter instanceof ExtPhpRs\Interface\ParentInterface,
    'Greeter should implement ParentInterface via #[php_impl_interface]');

// Test is_a() works for interface checking
assert(is_a($greeter, 'ExtPhpRs\Interface\ParentInterface'),
    'is_a() should recognize Greeter as ParentInterface');

// Test the class method
assert($greeter->getName() === 'World', 'Greeter should return correct name');

// Test type hinting with the interface
function greetWithInterface(ExtPhpRs\Interface\ParentInterface $obj): string {
    // The parentMethod is defined on the interface
    return $obj->parentMethod();
}

$result = greetWithInterface($greeter);
assert($result === 'Hello from World!', 'parentMethod should work via interface type hint');

// ============================================================================
// Test Feature 5: Short form implements syntax
// Using #[php(implements("\\InterfaceName"))] instead of verbose form
// ============================================================================

// Test ArrayAccess implementation using short form
assert(class_exists('ExtPhpRs\Interface\ShortFormArrayAccess'), 'ShortFormArrayAccess should exist');

$arr = new ExtPhpRs\Interface\ShortFormArrayAccess();
assert($arr instanceof ArrayAccess, 'ShortFormArrayAccess should implement ArrayAccess');

// Test ArrayAccess methods
assert(isset($arr[0]), 'offset 0 should exist');
assert(isset($arr[4]), 'offset 4 should exist');
assert(!isset($arr[5]), 'offset 5 should not exist');
assert($arr[0] === 10, 'offset 0 should be 10');
assert($arr[2] === 30, 'offset 2 should be 30');

$arr[1] = 99;
assert($arr[1] === 99, 'offset 1 should be 99 after set');

// Test Countable implementation using short form
assert(class_exists('ExtPhpRs\Interface\CountableTest'), 'CountableTest should exist');

$countable = new ExtPhpRs\Interface\CountableTest();
assert($countable instanceof Countable, 'CountableTest should implement Countable');

// Test count() function works
assert(count($countable) === 0, 'Empty CountableTest should have count 0');

$countable->add('one');
$countable->add('two');
$countable->add('three');
assert(count($countable) === 3, 'CountableTest with 3 items should have count 3');

// Test mixed implements syntax (explicit form + short form)
assert(class_exists('ExtPhpRs\Interface\MixedImplementsTest'), 'MixedImplementsTest should exist');

$mixed = new ExtPhpRs\Interface\MixedImplementsTest();
assert($mixed instanceof Iterator, 'MixedImplementsTest should implement Iterator (explicit form)');
assert($mixed instanceof Countable, 'MixedImplementsTest should implement Countable (short form)');
assert($mixed instanceof Traversable, 'MixedImplementsTest should implement Traversable');

// Test count() works
assert(count($mixed) === 3, 'MixedImplementsTest should have count 3');

// Test iteration works
$collected = [];
foreach ($mixed as $key => $value) {
    $collected[$key] = $value;
}
assert($collected === [0 => 10, 1 => 20, 2 => 30], 'MixedImplementsTest should iterate correctly');

// Test reflection shows both interfaces
$mixedReflection = new ReflectionClass(ExtPhpRs\Interface\MixedImplementsTest::class);
$interfaces = $mixedReflection->getInterfaceNames();
assert(in_array('Iterator', $interfaces), 'Reflection should show Iterator interface');
assert(in_array('Countable', $interfaces), 'Reflection should show Countable interface');
