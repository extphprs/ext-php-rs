//! Types and functions used for exporting Rust closures to PHP.

use std::collections::HashMap;

use crate::{
    args::{Arg, ArgParser},
    builders::{ClassBuilder, FunctionBuilder},
    class::{ClassEntryInfo, ClassMetadata, RegisteredClass},
    convert::{FromZval, IntoZval},
    describe::DocComments,
    exception::PhpException,
    flags::{DataType, MethodFlags},
    internal::property::PropertyInfo,
    types::Zval,
    zend::ExecuteData,
    zend_fastcall,
};

/// Class entry and handlers for Rust closures.
static CLOSURE_META: ClassMetadata<Closure> = ClassMetadata::new();

/// Wrapper around a Rust closure, which can be exported to PHP.
///
/// Closures can have up to 8 parameters, all must implement [`FromZval`], and
/// can return anything that implements [`IntoZval`]. Closures must have a
/// static lifetime, and therefore cannot modify any `self` references.
///
/// Internally, closures are implemented as a PHP class. A class `RustClosure`
/// is registered with an `__invoke` method:
///
/// ```php
/// <?php
///
/// class RustClosure {
///     public function __invoke(...$args): mixed {
///         // ...
///     }
/// }
/// ```
///
/// The Rust closure is then double boxed, firstly as a `Box<dyn Fn(...) ->
/// ...>` (depending on the signature of the closure) and then finally boxed as
/// a `Box<dyn PhpClosure>`. This is a workaround, as `PhpClosure` is not
/// generically implementable on types that implement `Fn(T, ...) -> Ret`. Make
/// a suggestion issue if you have a better idea of implementing this!.
///
/// When the `__invoke` method is called from PHP, the `invoke` method is called
/// on the `dyn PhpClosure`\ trait object, and from there everything is
/// basically the same as a regular PHP function.
pub struct Closure(Box<dyn PhpClosure>);

unsafe impl Send for Closure {}
unsafe impl Sync for Closure {}

impl Closure {
    /// Wraps a [`Fn`] or [`FnMut`] Rust closure into a type which can be
    /// returned to PHP.
    ///
    /// The closure can accept up to 8 arguments which implement [`IntoZval`],
    /// and can return any type which implements [`FromZval`]. The closure
    /// must have a static lifetime, so cannot reference `self`.
    ///
    /// # Parameters
    ///
    /// * `func` - The closure to wrap. Should be boxed in the form `Box<dyn
    ///   Fn[Mut](...) -> ...>`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ext_php_rs::closure::Closure;
    ///
    /// let closure = Closure::wrap(Box::new(|name| {
    ///     format!("Hello {}", name)
    /// }) as Box<dyn Fn(String) -> String>);
    /// ```
    pub fn wrap<T>(func: T) -> Self
    where
        T: PhpClosure + 'static,
    {
        Self(Box::new(func) as Box<dyn PhpClosure>)
    }

    /// Wraps a [`FnOnce`] Rust closure into a type which can be returned to
    /// PHP. If the closure is called more than once from PHP, an exception
    /// is thrown.
    ///
    /// The closure can accept up to 8 arguments which implement [`IntoZval`],
    /// and can return any type which implements [`FromZval`]. The closure
    /// must have a static lifetime, so cannot reference `self`.
    ///
    /// # Parameters
    ///
    /// * `func` - The closure to wrap. Should be boxed in the form `Box<dyn
    ///   FnOnce(...) -> ...>`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ext_php_rs::closure::Closure;
    ///
    /// let name: String = "Hello world".into();
    /// let closure = Closure::wrap_once(Box::new(|| {
    ///     name
    /// }) as Box<dyn FnOnce() -> String>);
    /// ```
    pub fn wrap_once<T>(func: T) -> Self
    where
        T: PhpOnceClosure + 'static,
    {
        func.into_closure()
    }

    /// Builds the class entry for [`Closure`], registering it with PHP. This
    /// function should only be called once inside your module startup
    /// function.
    ///
    /// If the class has already been built, this function returns early without
    /// doing anything. This allows for safe repeated calls in test environments.
    ///
    /// # Panics
    ///
    /// Panics if the `RustClosure` PHP class cannot be registered.
    pub fn build() {
        if CLOSURE_META.has_ce() {
            return;
        }

        ClassBuilder::new("RustClosure")
            .method(
                FunctionBuilder::new("__invoke", Self::invoke)
                    .not_required()
                    .arg(Arg::new("args", DataType::Mixed).is_variadic())
                    .returns(DataType::Mixed, false, true),
                MethodFlags::Public,
            )
            .object_override::<Self>()
            .registration(|ce| CLOSURE_META.set_ce(ce))
            .register()
            .expect("Failed to build `RustClosure` PHP class.");
    }

    zend_fastcall! {
        /// External function used by the Zend interpreter to call the closure.
        extern "C" fn invoke(ex: &mut ExecuteData, ret: &mut Zval) {
            let (parser, this) = ex.parser_method::<Self>();
            let this = this.expect("Internal closure function called on non-closure class");

            this.0.invoke(parser, ret);
        }
    }
}

impl RegisteredClass for Closure {
    const CLASS_NAME: &'static str = "RustClosure";

    const BUILDER_MODIFIER: Option<fn(ClassBuilder) -> ClassBuilder> = None;
    const EXTENDS: Option<ClassEntryInfo> = None;
    const IMPLEMENTS: &'static [ClassEntryInfo] = &[];

    fn get_metadata() -> &'static ClassMetadata<Self> {
        &CLOSURE_META
    }

    fn get_properties<'a>() -> HashMap<&'static str, PropertyInfo<'a, Self>> {
        HashMap::new()
    }

    fn method_builders() -> Vec<(FunctionBuilder<'static>, MethodFlags)> {
        unimplemented!()
    }

    fn constructor() -> Option<crate::class::ConstructorMeta<Self>> {
        None
    }

    fn constants() -> &'static [(
        &'static str,
        &'static dyn crate::convert::IntoZvalDyn,
        DocComments,
    )] {
        unimplemented!()
    }
}

class_derives!(Closure);

/// Implemented on types which can be used as PHP closures.
///
/// Types must implement the `invoke` function which will be called when the
/// closure is called from PHP. Arguments must be parsed from the
/// [`ExecuteData`] and the return value is returned through the [`Zval`].
///
/// This trait is automatically implemented on functions with up to 8
/// parameters.
#[allow(clippy::missing_safety_doc)]
pub unsafe trait PhpClosure {
    /// Invokes the closure.
    fn invoke<'a>(&'a mut self, parser: ArgParser<'a, '_>, ret: &mut Zval);
}

/// Implemented on [`FnOnce`] types which can be used as PHP closures. See
/// [`Closure`].
///
/// Internally, this trait should wrap the [`FnOnce`] closure inside a [`FnMut`]
/// closure, and prevent the user from calling the closure more than once.
pub trait PhpOnceClosure {
    /// Converts the Rust [`FnOnce`] closure into a [`FnMut`] closure, and then
    /// into a PHP closure.
    fn into_closure(self) -> Closure;
}

unsafe impl<R> PhpClosure for Box<dyn Fn() -> R>
where
    R: IntoZval,
{
    fn invoke(&mut self, _: ArgParser, ret: &mut Zval) {
        if let Err(e) = self().set_zval(ret, false) {
            let _ = PhpException::default(format!("Failed to return closure result to PHP: {e}"))
                .throw();
        }
    }
}

unsafe impl<R> PhpClosure for Box<dyn FnMut() -> R>
where
    R: IntoZval,
{
    fn invoke(&mut self, _: ArgParser, ret: &mut Zval) {
        if let Err(e) = self().set_zval(ret, false) {
            let _ = PhpException::default(format!("Failed to return closure result to PHP: {e}"))
                .throw();
        }
    }
}

impl<R> PhpOnceClosure for Box<dyn FnOnce() -> R>
where
    R: IntoZval + 'static,
{
    fn into_closure(self) -> Closure {
        let mut this = Some(self);

        Closure::wrap(Box::new(move || {
            let Some(this) = this.take() else {
                let _ = PhpException::default(
                    "Attempted to call `FnOnce` closure more than once.".into(),
                )
                .throw();
                return Option::<R>::None;
            };

            Some(this())
        }) as Box<dyn FnMut() -> Option<R>>)
    }
}

macro_rules! php_closure_impl {
    ($($gen: ident),*) => {
        php_closure_impl!(Fn; $($gen),*);
        php_closure_impl!(FnMut; $($gen),*);

        impl<$($gen),*, Ret> PhpOnceClosure for Box<dyn FnOnce($($gen),*) -> Ret>
        where
            $(for<'a> $gen: FromZval<'a> + 'static,)*
            Ret: IntoZval + 'static,
        {
            fn into_closure(self) -> Closure {
                let mut this = Some(self);

                Closure::wrap(Box::new(move |$($gen),*| {
                    let Some(this) = this.take() else {
                        let _ = PhpException::default(
                            "Attempted to call `FnOnce` closure more than once.".into(),
                        )
                        .throw();
                        return Option::<Ret>::None;
                    };

                    Some(this($($gen),*))
                }) as Box<dyn FnMut($($gen),*) -> Option<Ret>>)
            }
        }
    };

    ($fnty: ident; $($gen: ident),*) => {
        unsafe impl<$($gen),*, Ret> PhpClosure for Box<dyn $fnty($($gen),*) -> Ret>
        where
            $(for<'a> $gen: FromZval<'a>,)*
            Ret: IntoZval
        {
            fn invoke(&mut self, parser: ArgParser, ret: &mut Zval) {
                $(
                    let mut $gen = Arg::new(stringify!($gen), $gen::TYPE);
                )*

                let parser = parser
                    $(.arg(&mut $gen))*
                    .parse();

                if parser.is_err() {
                    return;
                }

                let result = self(
                    $(
                        match $gen.consume() {
                            Ok(val) => val,
                            _ => {
                                let _ = PhpException::default(concat!("Invalid parameter type for `", stringify!($gen), "`.").into()).throw();
                                return;
                            }
                        }
                    ),*
                );

                if let Err(e) = result.set_zval(ret, false) {
                    let _ = PhpException::default(format!("Failed to return closure result to PHP: {}", e)).throw();
                }
            }
        }
    };
}

php_closure_impl!(A);
php_closure_impl!(A, B);
php_closure_impl!(A, B, C);
php_closure_impl!(A, B, C, D);
php_closure_impl!(A, B, C, D, E);
php_closure_impl!(A, B, C, D, E, F);
php_closure_impl!(A, B, C, D, E, F, G);
php_closure_impl!(A, B, C, D, E, F, G, H);
