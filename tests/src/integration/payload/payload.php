<?php

declare(strict_types=1);

final class CollectOnDestruct
{
    public function __destruct()
    {
        gc_collect_cycles();
    }
}

$holder = new NestedHolder();
$holder->add(0, static fn() => $holder);
$holder->add(
    1,
    ( static function (): Closure {
        $trigger = new CollectOnDestruct();
        return static fn() => $trigger;
    } )()
);
$alias = $holder;
unset($alias);
$holder->reset();

$pair = new TwoFields();
$pair->add(
    static fn() => $pair,
    ( static function (): Closure {
        $trigger = new CollectOnDestruct();
        return static fn() => $trigger;
    } )()
);
$alias = $pair;
unset($alias);
$pair->reset();

function thrown(Closure $call): string
{
    try {
        $call();
    } catch (Throwable $throwable) {
        return $throwable::class . ': ' . $throwable->getMessage();
    }

    return 'nothing thrown';
}

$reentrant = new Reentrant();
$reentrant->setCallback($reentrant->bump(...));
$message = thrown($reentrant->callWhileModifying(...));
assert($message === 'Error: Cannot modify object of class Reentrant while it is in use', 'nested modification');
$reentrant->setCallback($reentrant->count(...));
$message = thrown($reentrant->callWhileModifying(...));
assert(
    $message === 'Error: Cannot use object of class Reentrant while one of its methods modifies it',
    'read during a modification'
);
assert($reentrant->callWhileReading() === 2, 'a read during a read was refused');

$message = thrown(static fn() => payload_pair($reentrant, $reentrant));
assert($message === 'TypeError: Invalid value given for argument `second`.', 'same object, mutable and shared');
$message = thrown(static fn() => $reentrant->absorb($reentrant));
assert($message === 'TypeError: Invalid value given for argument `other`.', 'receiver passed as a shared argument');
assert($reentrant->absorb(new Reentrant()) === 2, 'a shared argument of another object was refused');

assert(payload_extract($reentrant) === '2', 'try_extract refused a free object');
assert(payload_extract(null) === 'null', 'try_extract refused null');
assert(
    str_starts_with(payload_extract(new stdClass()), 'could not convert value of type '),
    'try_extract accepted another class'
);
$reentrant->setCallback(static fn() => payload_extract($reentrant));
assert(
    $reentrant->callWhileModifying() === 'Cannot use object of class Reentrant while one of its methods modifies it',
    'try_extract hid the borrow conflict'
);

$emitter = new Emitter();
$emitter->on(static function () use ($emitter): void {
    $emitter->on(static fn() => null);
});
$emitter->emit();
assert($emitter->count() === 2, 'a listener could not register another listener during emit');

final class SkipsParentConstructor extends Reentrant
{
    public function __construct()
    {
    }
}

$message = thrown(static fn() => ( new SkipsParentConstructor() )->count());
assert($message === 'Error: Object of class Reentrant is not initialized', 'uninitialized object');

final class DropProbe
{
    public static string $seen = 'not dropped';
}

$dropped = new CallsBackOnDrop();
$weakDropped = WeakReference::create($dropped);
$dropped->setCallback(static function () use ($weakDropped): void {
    try {
        DropProbe::$seen = $weakDropped->get()?->label() ?? 'gone';
    } catch (Error $error) {
        DropProbe::$seen = $error->getMessage();
    }
});
unset($dropped);
assert(
    DropProbe::$seen === 'Object of class CallsBackOnDrop is not initialized',
    'a Drop that called back into the object did not see it uninitialized'
);
