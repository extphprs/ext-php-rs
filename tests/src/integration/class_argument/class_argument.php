<?php

declare(strict_types=1);

use ExtPhpRs\ClassArgument\Peer;

function thrown(Closure $call): string
{
    try {
        $call();
    } catch (Throwable $throwable) {
        return $throwable::class . ': ' . $throwable->getMessage();
    }

    return 'nothing thrown';
}

final class ArgCounterWithoutParent extends ArgCounter
{
    public function __construct()
    {
    }
}

$first = new ArgCounter(2, 'first');
$second = new ArgCounter(3, 'second');

assert($first->add($second) === 5, '&T parameter on a &mut self method');
assert($first->give($second) === 8, '&mut T parameter on a &self method');
assert($second->count() === 8, 'the &mut T parameter modified the object');
assert($first->addMaybe(null) === 5, 'Option<&T> parameter given null');
assert($first->addMaybe() === 5, 'Option<&T> parameter omitted');
assert($first->addMaybe($second) === 13, 'Option<&T> parameter given an object');
assert($first->giveMaybe(null) === -1, 'Option<&mut T> parameter given null');
assert($first->giveMaybe($second) === 13, 'Option<&mut T> parameter given an object');
assert(ArgCounter::sum($first, $second) === 18, '&T parameters on a static method');
assert(ArgCounter::sum($first, $first) === 10, 'the same object as two &T parameters');
assert(( new ArgView($second) )->count() === 13, '&T parameter on a constructor');
assert($first->peerTotal($second) === 18, '&T parameter on an interface method');
assert($first instanceof Peer);
assert(arg_counter_pair($first, $second) === 18, '&mut T and &T parameters on a function');
assert(arg_counter_label($second) === 'second', 'a result that borrows a &T parameter');
assert(arg_counter_total($first, $second) === 31, 'variadic ClassRef parameters');

assert(thrown(fn() => $first->add($first))
=== 'Error: Cannot use object of class ArgCounter while one of its methods modifies it', 'the receiver of a &mut self method passed as a &T parameter');
assert(thrown(fn() => $first->give($first))
=== 'Error: Cannot modify object of class ArgCounter while it is in use', 'the receiver of a &self method passed as a &mut T parameter');
assert(thrown(fn() => arg_counter_pair($first, $first))
=== 'Error: Cannot use object of class ArgCounter while one of its methods modifies it', 'the same object as a &mut T and a &T parameter');
assert(thrown(fn() => arg_counter_read($first, $first->bump(...)))
=== 'Error: Cannot modify object of class ArgCounter while it is in use', 'a callback that modifies an object borrowed as a &T parameter');
$first->setCallback($first->bump(...));
assert(thrown(fn() => $second->runCallbackOf($first))
=== 'Error: Cannot modify object of class ArgCounter while it is in use', 'a callback stored in a &T parameter that modifies it');
assert(arg_counter_read($first, $first->count(...)) === 18, 'a callback that reads a &T parameter');

assert(thrown(fn() => $first->add(new ArgCounterWithoutParent()))
=== 'Error: Object of class ArgCounter is not initialized', 'an object without a Rust value as a &T parameter');
assert(thrown(fn() => $first->add(new stdClass()))
=== 'TypeError: Invalid value given for argument `other`.', 'an object of another class as a &T parameter');
assert(thrown(fn() => $first->add(null))
=== 'TypeError: Invalid value given for argument `other`.', 'null as a &T parameter');
assert($first->count() === 18, 'the failed calls left the object unchanged');

assert($first->addAll($second, $second) === 44, '&T and variadic ClassRef parameters on a &mut self method');
assert($first->bumpWhileCalling(fn() => thrown(fn() => arg_counter_total($second, $first)))
=== 'Error: Cannot use object of class ArgCounter while one of its methods modifies it', 'an object that a method modifies in a variadic ClassRef parameter');
assert(thrown(fn() => arg_counter_total($second, new ArgCounterWithoutParent()))
=== 'Error: Object of class ArgCounter is not initialized', 'an object without a Rust value in a variadic ClassRef parameter');
assert(thrown(fn() => arg_counter_total($second, new stdClass()))
=== 'TypeError: Invalid value given for argument `counters`.', 'an object of another class in a variadic ClassRef parameter');
assert(thrown(fn() => $first->addAll($first))
=== 'Error: Cannot use object of class ArgCounter while one of its methods modifies it', 'the receiver of a &mut self method passed as a &T parameter before a variadic');
assert($first->count() === 45, 'the failed variadic calls left the object unchanged');

$view = new ArgView($second);
$view->source = $first;
assert($view->count() === 45, 'a &T parameter on a property setter');
assert($first->bumpWhileCalling(fn() => thrown(fn() => $view->source = $first))
=== 'Error: Cannot use object of class ArgCounter while one of its methods modifies it', 'an object that a method modifies given to a &T setter');
assert(
    thrown(fn() => $view->source = new stdClass())
    === 'Exception: Unable to convert property value into required type.',
    'an object of another class given to a &T setter'
);
assert($view->count() === 45, 'the failed setter calls left the object unchanged');
