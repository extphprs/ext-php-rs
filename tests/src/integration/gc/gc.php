<?php

declare(strict_types=1);

final class GcChild extends GcHolder
{
    public ?object $edge = null;
}

#[AllowDynamicProperties]
final class GcDynamicChild extends GcHolder
{
    public ?object $edge = null;
}

final class GcUninitializedChild extends GcHolder
{
    public ?object $edge = null;

    public function __construct()
    {
    }
}

final class ReadsHolderOnDestruct
{
    public static mixed $seen = 'not destructed';

    public function __construct(
        private readonly GcHolder $holder
    ) {
    }

    public function __destruct()
    {
        self::$seen = $this->holder->callback;
    }
}

function assert_collected(string $case, Closure $build): void
{
    $dropped = gc_holder_dropped();
    $weak = $build();
    gc_collect_cycles();
    assert($weak->get() === null, "{$case}: the cycle was not collected");
    assert(gc_holder_dropped() === ( $dropped + 1 ), "{$case}: the Rust payload was not dropped");
}

assert_collected('Zval field', static function (): WeakReference {
    $holder = new GcHolder();
    $holder->setCallback(static fn() => $holder);
    return WeakReference::create($holder);
});

assert_collected('Vec<Zval> field with the same closure twice', static function (): WeakReference {
    $holder = new GcHolder();
    $callback = static fn() => $holder;
    $holder->addListener($callback);
    $holder->addListener($callback);
    return WeakReference::create($holder);
});

assert_collected('Vec<Zval> field that grows the engine buffer', static function (): WeakReference {
    $holder = new GcHolder();
    for ($i = 0; $i < 200; $i++) {
        $holder->addListener(static fn() => $holder);
    }
    return WeakReference::create($holder);
});

assert_collected('HashMap<String, Zval> field', static function (): WeakReference {
    $holder = new GcHolder();
    $holder->setNamed('first', static fn() => $holder);
    return WeakReference::create($holder);
});

assert_collected('owned ZendCallable field', static function (): WeakReference {
    $holder = new GcHolder();
    $holder->setCallable(static fn() => $holder);
    return WeakReference::create($holder);
});

assert_collected('ZBox<ZendObject> field', static function (): WeakReference {
    $holder = new GcHolder();
    $holder->child()->parent = $holder;
    return WeakReference::create($holder);
});

assert_collected('getter value kept in the declared slot', static function (): WeakReference {
    $holder = new GcHolder();
    $holder->setCallback(static fn() => $holder);
    assert($holder->callback instanceof Closure);
    return WeakReference::create($holder);
});

assert_collected('declared property of a subclass with Rust roots', static function (): WeakReference {
    $holder = new GcChild();
    $holder->setCallback(static fn() => 'not a cycle');
    $edge = new stdClass();
    $edge->back = $holder;
    $holder->edge = $edge;
    return WeakReference::create($holder);
});

assert_collected('materialized properties table with Rust roots', static function (): WeakReference {
    $holder = new GcDynamicChild();
    $holder->setCallback(static fn() => 'not a cycle');
    $edge = new stdClass();
    $edge->back = $holder;
    $holder->dynamic = $edge;
    assert(array_key_exists('dynamic', get_object_vars($holder)));
    return WeakReference::create($holder);
});

$uninitialized = new GcUninitializedChild();
$uninitialized->edge = $uninitialized;
$weakUninitialized = WeakReference::create($uninitialized);
unset($uninitialized);
gc_collect_cycles();
assert($weakUninitialized->get() === null, 'a subclass without a Rust payload was not collected');

$holder = new GcHolder();
$callback = static fn() => $holder;
$holder->setCallback($callback);
$holder->addListener($callback);
$holder->setNamed('first', $callback);
$holder->setCallable($callback);
$weak = WeakReference::create($holder);
unset($holder);
gc_collect_cycles();
assert($weak->get() !== null, 'a cycle held from outside was collected');
assert($callback() === $weak->get(), 'the closure lost its captured holder');
assert($weak->get()->label() === 'holder', 'the Rust payload was freed while in use');
unset($callback);
gc_collect_cycles();
assert($weak->get() === null, 'the cycle was not collected after the outside reference went away');

$dropped = gc_holder_dropped();
for ($i = 0; $i < 1000; $i++) {
    $holder = new GcHolder();
    $holder->setCallback(static fn() => $holder);
}
unset($holder);
gc_collect_cycles();
assert(gc_holder_dropped() === ( $dropped + 1000 ), 'some of the 1000 cycles were not collected');

$holder = new GcHolder();
$holder->setCallback(new ReadsHolderOnDestruct($holder));
$holder->setCallback(static fn() => 'replaced');
assert(ReadsHolderOnDestruct::$seen === null, 'a destructor saw the field value it was released from');
unset($holder);
gc_collect_cycles();
