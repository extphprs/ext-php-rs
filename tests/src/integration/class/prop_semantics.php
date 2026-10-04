<?php

$notices = [];
set_error_handler(static function (int $errno, string $message) use (&$notices): bool {
    $notices[] = $message;

    return true;
});

function expect_error(callable $fn, string $needle): void
{
    try {
        $fn();
    } catch (Error $e) {
        assert(str_contains($e->getMessage(), $needle), $e->getMessage());

        return;
    }
    assert(false, "expected Error containing {$needle}");
}

$o = new TestPropSemantics(1);
$o->num += 4;
assert($o->rustNum() === 5);
$o->num++;
++$o->num;
$o->num--;
assert($o->rustNum() === 6);
$o->label .= 'abc';
$o->label .= 'def';
assert($o->rustLabel() === 'abcdef');
$o->label ??= 'ignored';
assert($o->rustLabel() === 'abcdef');

$name = 'num';
$o->$name += 10;
$o->$name++;
assert($o->rustNum() === 17);

class PlainNum
{
    public int $num = 0;
}

class NoBacking extends TestPropSemantics
{
    public function __construct()
    {
    }
}

class LateInit extends TestPropSemantics
{
    public mixed $before;

    public function __construct()
    {
        $this->before = $this->num;
        parent::__construct(7);
        $this->num += 1;
    }
}

class MaybeInit extends TestPropSemantics
{
    public function __construct(bool $init)
    {
        if ($init) {
            parent::__construct(5);
        }
    }
}

function bump_num(object $x): void
{
    $x->num += 1;
}

function ref_num(object $x): void
{
    $r = &$x->num;
    $r = 99;
}

$plain = new PlainNum();
$unbacked = new NoBacking();
$backed = new TestPropSemantics(10);
foreach ([$plain, $unbacked, $backed, $plain, $unbacked, $backed] as $x) {
    bump_num($x);
}
assert($plain->num === 2);
assert($backed->rustNum() === 12);

$notices = [];
foreach ([$plain, $unbacked, $backed] as $x) {
    ref_num($x);
}
assert($plain->num === 99);
assert($backed->rustNum() === 12);
assert(count($notices) === 1, var_export($notices, true));
assert(str_contains($notices[0], 'Indirect modification of overloaded property TestPropSemantics::$num'));

$late = new LateInit();
assert($late->before === null);
assert($late->rustNum() === 8);
assert($late->num === 8);

$notices = [];
$o->list[] = 1;
assert($o->rustList() === []);
assert(count($notices) === 1, var_export($notices, true));
assert(str_contains($notices[0], 'Indirect modification of overloaded property TestPropSemantics::$list'));

expect_error(static function () use ($o): void {
    $x = 1;
    $o->num = &$x;
}, 'Cannot assign by reference to overloaded object');
expect_error(static function () use ($o): void {
    $o->secret[] = 1;
}, 'Cannot access private property');

expect_error(static function () use ($o): void {
    unset($o->num);
}, 'Cannot unset property TestPropSemantics::$num');
expect_error(static function () use ($o): void {
    unset($o->secret);
}, 'Cannot access private property');
assert($o->rustNum() === 17);
assert(isset($o->num));
assert(property_exists($o, 'num'));

$a = new TestPropSemantics(3);
$b = new TestPropSemantics(3);
assert(( $a <=> $b ) === 0);
$a->bump();
assert(( $a <=> $b ) !== 0);
$c = clone $b;
assert(( $c <=> $b ) === 0);
assert(( new NoBacking() <=> new TestPropSemantics(3) ) !== 0);

$m1 = new MaybeInit(true);
$m2 = new MaybeInit(false);
$m2->num = 5;
assert(( $m1 <=> $m2 ) === 0);

$f = new TestPropSemantics(1);
$f->bump();
$seen = [];
foreach ($f as $k => $v) {
    $seen[$k] = $v;
}
assert($seen['num'] === 101, var_export($seen, true));
assert(!array_key_exists('secret', $seen));

class Reenter
{
    public function __construct(
        private TestPropReentry $owner
    ) {
    }

    public function __destruct()
    {
        get_object_vars($this->owner);
        $this->owner->touch();
    }
}

$r = new TestPropReentry();
$r->replaceHolder(new Reenter($r));
get_object_vars($r);
$r->replaceHolder(new Reenter($r));
get_object_vars($r);
assert($r->hits() === 1);
$r->replaceHolder(null);
