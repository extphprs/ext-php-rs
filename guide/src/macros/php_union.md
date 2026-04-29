# `PhpUnion` Derive Macro

The `#[derive(PhpUnion)]` macro lets a Rust enum stand in for a PHP union type
on `#[php_function]` and `#[php_impl]` signatures. Each variant must
newtype-wrap exactly one field; the inner type must implement `IntoZval` and
`FromZval`.

## What it emits

The derive emits three impls on the enum:

- `impl PhpUnion` whose `union_types()` returns
  `PhpType::Union(vec![<T1 as IntoZval>::TYPE, <T2 as IntoZval>::TYPE, ...])`.
- `impl IntoZval` whose `set_zval` dispatches on the variant and whose
  `php_type()` override delegates to `<Self as PhpUnion>::union_types()`. This
  is what causes the function macro to register the right `int|string` shape.
- `impl FromZval` whose `from_zval` tries each variant's inner type in
  declaration order. The same `php_type()` override applies.

## Variant shapes

Only newtype variants are accepted in the first iteration:

```rust,ignore
#[derive(PhpUnion)]
pub enum IntOrString {
    Int(i64),
    Str(String),
}
```

The derive rejects, with a span on the offending variant:

- unit variants (`None`),
- struct variants (`{ a: i32 }`),
- multi-field tuple variants (`(i32, String)`).

Generics on the enum are also rejected. Both restrictions can be lifted in a
follow-up if demand surfaces.

## Variant ordering

`FromZval` walks variants in declaration order and stops on the first match.
Order matters when two inner types accept the same PHP value — for example, a
`String` variant before a `ParsedStr(String)` variant would always win even
when the zval is a numeric string. List the more specific variant first.

## Example

```rust,no_run,ignore
# #![cfg_attr(windows, feature(abi_vectorcall))]
# extern crate ext_php_rs;
use ext_php_rs::prelude::*;

#[derive(PhpUnion)]
pub enum IntOrString {
    Int(i64),
    Str(String),
}

#[php_function]
pub fn echo_either(value: IntOrString) -> IntOrString {
    value
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.function(wrap_function!(echo_either))
}
# fn main() {}
```

Use from PHP:

```php
echo_either(42);     // returns int(42), takes the IntOrString::Int branch
echo_either('hi');   // returns string(2) "hi", takes the IntOrString::Str branch
```

PHP's reflection sees the registered union on both sides:

```php
$rf = new ReflectionFunction('echo_either');
$param = $rf->getParameters()[0]->getType(); // ReflectionUnionType: int|string
$ret   = $rf->getReturnType();               // ReflectionUnionType: int|string
```

## Relationship to `#[derive(ZvalConvert)]`

`#[derive(ZvalConvert)]` on an enum produces a similar variant-dispatching
`IntoZval`/`FromZval`, but registers the parameter as `mixed` because it has
no way to express the union at registration time. `#[derive(PhpUnion)]`
overrides `php_type()` so the function macro registers `int|string`,
`int|string|null`, etc. as appropriate.

If the enum is only ever consumed from Rust (never crossing into PHP through
a registered function), `ZvalConvert` is enough. The moment you want PHP
reflection or strict-types coercion to see the actual union members, prefer
`PhpUnion`.

## Relationship to `#[php(types = "...")]`

The slice-06 attribute `#[php(types = "int|string")]` is the explicit override
when a Rust signature is `&Zval` or otherwise can't carry the type information
in the type system. `PhpUnion` is the type-driven path: the type itself
encodes the union, so the function signature is plain Rust and the macro
infers the PHP shape from the derive. Use the attribute when you want to
accept a raw `Zval` and inspect it manually; use `PhpUnion` when the variants
already carry the right Rust types.
