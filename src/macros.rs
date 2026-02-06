//! Macros for interacting with PHP, mainly when the function takes variadic
//! arguments. Unfortunately, this is the best way to handle these.
//! Note that most of these will introduce unsafe into your code base.

/// Starts the PHP extension information table displayed when running
/// `phpinfo();` Must be run *before* rows are inserted into the table.
#[macro_export]
macro_rules! info_table_start {
    () => {
        unsafe { $crate::ffi::php_info_print_table_start() };
    };
}

/// Ends the PHP extension information table. Must be run *after* all rows have
/// been inserted into the table.
#[macro_export]
macro_rules! info_table_end {
    () => {
        unsafe { $crate::ffi::php_info_print_table_end() }
    };
}

/// Sets the header for the PHP extension information table. Takes as many
/// string arguments as required.
#[macro_export]
macro_rules! info_table_header {
    ($($element:expr),*) => {$crate::_info_table_row!(php_info_print_table_header, $($element),*)};
}

/// Adds a row to the PHP extension information table. Takes as many string
/// arguments as required.
#[macro_export]
macro_rules! info_table_row {
    ($($element:expr),*) => {$crate::_info_table_row!(php_info_print_table_row, $($element),*)};
}

/// INTERNAL: Calls a variadic C function with the number of parameters, then
/// following with the parameters.
#[doc(hidden)]
#[macro_export]
macro_rules! _info_table_row {
    ($fn: ident, $($element: expr),*) => {
        unsafe {
            $crate::ffi::$fn($crate::_info_table_row!(@COUNT; $($element),*) as i32, $(::std::ffi::CString::new($element).unwrap().as_ptr()),*);
        }
    };

    (@COUNT; $($element: expr),*) => {
        <[()]>::len(&[$($crate::_info_table_row![@SUBST; $element]),*])
    };
    (@SUBST; $_: expr) => { () };
}

/// Attempts to call a given PHP callable.
///
/// # Parameters
///
/// * `$fn` - The 'function' to call. Can be an [`Arg`] or a [`Zval`].
/// * ...`$param` - The parameters to pass to the function. Must be able to be
///   converted into a [`Zval`].
///
/// [`Arg`]: crate::args::Arg
/// [`Zval`]: crate::types::Zval
#[macro_export]
macro_rules! call_user_func {
    ($fn: expr) => {
        $fn.try_call(vec![])
    };

    ($fn: expr, $($param: expr),*) => {
        $fn.try_call(vec![$(&$param),*])
    };
}

/// Parses a given list of arguments using the [`ArgParser`] class.
///
/// # Examples
///
/// This example parses all of the arguments. If one is invalid, execution of
/// the function will stop at the `parse_args!` macro invocation. The user is
/// notified via PHP's argument parsing system.
///
/// In this case, all of the arguments are required.
///
/// ```
/// # #[macro_use] extern crate ext_php_rs;
/// use ext_php_rs::{
///     parse_args,
///     args::Arg,
///     flags::DataType,
///     zend::ExecuteData,
///     types::Zval,
/// };
///
/// pub extern "C" fn example_fn(execute_data: &mut ExecuteData, _: &mut Zval) {
///     let mut x = Arg::new("x", DataType::Long);
///     let mut y = Arg::new("y", DataType::Long);
///     let mut z = Arg::new("z", DataType::Long);
///
///     parse_args!(execute_data, x, y, z);
/// }
/// ```
///
/// This example is similar to the one above, apart from the fact that the `z`
/// argument is not required. Note the semicolon separating the first two
/// arguments from the second.
///
/// ```
/// use ext_php_rs::{
///     parse_args,
///     args::Arg,
///     flags::DataType,
///     zend::ExecuteData,
///     types::Zval,
/// };
///
/// pub extern "C" fn example_fn(execute_data: &mut ExecuteData, _: &mut Zval) {
///     let mut x = Arg::new("x", DataType::Long);
///     let mut y = Arg::new("y", DataType::Long);
///     let mut z = Arg::new("z", DataType::Long);
///
///     parse_args!(execute_data, x, y; z);
/// }
/// ```
///
/// [`ArgParser`]: crate::args::ArgParser
#[macro_export]
macro_rules! parse_args {
    ($ed: expr, $($arg: expr),*) => {{
        let parser = $ed.parser()
            $(.arg(&mut $arg))*
            .parse();
        if parser.is_err() {
            return;
        }
    }};

    ($ed: expr, $($arg: expr),* ; $($opt: expr),*) => {{
        let parser = $ed.parser()
            $(.arg(&mut $arg))*
            .not_required()
            $(.arg(&mut $opt))*
            .parse();
        if parser.is_err() {
            return;
        }
    }};
}

/// Throws an exception and returns from the current function.
///
/// Wraps the [`throw`] function by inserting a `return` statement after
/// throwing the exception.
///
/// [`throw`]: crate::exception::throw
///
/// # Examples
///
/// ```
/// use ext_php_rs::{
///     throw,
///     zend::{ce, ClassEntry, ExecuteData},
///     types::Zval,
/// };
///
/// pub extern "C" fn example_fn(execute_data: &mut ExecuteData, _: &mut Zval) {
///     let something_wrong = true;
///     if something_wrong {
///         throw!(ce::exception(), "Something is wrong!");
///     }
///
///     assert!(false); // This will not run.
/// }
/// ```
#[macro_export]
macro_rules! throw {
    ($ex: expr, $reason: expr) => {
        $crate::exception::throw($ex, $reason);
        return;
    };
}

/// Implements a set of traits required to convert types that implement
/// [`RegisteredClass`] to and from [`ZendObject`]s and [`Zval`]s. Generally,
/// this macro should not be called directly, as it is called on any type that
/// uses the [`php_class`] macro.
///
/// The following traits are implemented:
///
/// * `FromZendObject for &'a T`
/// * `FromZendObjectMut for &'a mut T`
/// * `FromZval for &'a T`
/// * `FromZvalMut for &'a mut T`
/// * `IntoZendObject for T`
/// * `IntoZval for T`
///
/// These implementations are required while we wait on the stabilisation of
/// specialisation.
///
/// # Examples
///
/// ```
/// # use ext_php_rs::{convert::{IntoZval, FromZval, IntoZvalDyn}, types::{Zval, ZendObject}, class::{RegisteredClass, ConstructorMeta, ClassEntryInfo}, builders::{ClassBuilder, FunctionBuilder}, zend::ClassEntry, flags::{ClassFlags, MethodFlags}, internal::property::PropertyInfo, describe::DocComments};
/// use ext_php_rs::class_derives;
///
/// struct Test {
///     a: i32,
///     b: i64
/// }
///
/// impl RegisteredClass for Test {
///     const CLASS_NAME: &'static str = "Test";
///
///     const BUILDER_MODIFIER: Option<fn(ClassBuilder) -> ClassBuilder> = None;
///     const EXTENDS: Option<ClassEntryInfo> = None;
///     const IMPLEMENTS: &'static [ClassEntryInfo] =  &[];
///     const FLAGS: ClassFlags = ClassFlags::empty();
///     const DOC_COMMENTS: DocComments = &[];
///
///     fn get_metadata() -> &'static ext_php_rs::class::ClassMetadata<Self> {
///         todo!()
///     }
///
///     fn get_properties<'a>(
///     ) -> std::collections::HashMap<&'static str, PropertyInfo<'a, Self>>
///     {
///         todo!()
///     }
///
///     fn method_builders() -> Vec<(FunctionBuilder<'static>, MethodFlags)> {
///         todo!()
///     }
///
///     fn constructor() -> Option<ConstructorMeta<Self>> {
///         todo!()
///     }
///
///     fn constants() -> &'static [(&'static str, &'static dyn IntoZvalDyn, DocComments)] {
///         todo!()
///     }
/// }
///
/// class_derives!(Test);
///
/// fn into_zval_test() -> Zval {
///     let x = Test { a: 5, b: 10 };
///     x.into_zval(false).unwrap()
/// }
///
/// fn from_zval_test<'a>(zv: &'a Zval) -> &'a Test {
///     <&Test>::from_zval(zv).unwrap()
/// }
/// ```
///
/// [`RegisteredClass`]: crate::class::RegisteredClass
/// [`ZendObject`]: crate::types::ZendObject
/// [`Zval`]: crate::types::Zval
/// [`php_class`]: crate::php_class
#[macro_export]
macro_rules! class_derives {
    ($type: ty) => {
        impl<'a> $crate::convert::FromZendObject<'a> for &'a $type {
            #[inline]
            fn from_zend_object(obj: &'a $crate::types::ZendObject) -> $crate::error::Result<Self> {
                let obj = $crate::types::ZendClassObject::<$type>::from_zend_obj(obj)
                    .ok_or($crate::error::Error::InvalidScope)?;
                Ok(&**obj)
            }
        }

        impl<'a> $crate::convert::FromZendObjectMut<'a> for &'a mut $type {
            #[inline]
            fn from_zend_object_mut(
                obj: &'a mut $crate::types::ZendObject,
            ) -> $crate::error::Result<Self> {
                let obj = $crate::types::ZendClassObject::<$type>::from_zend_obj_mut(obj)
                    .ok_or($crate::error::Error::InvalidScope)?;
                Ok(&mut **obj)
            }
        }

        impl<'a> $crate::convert::FromZval<'a> for &'a $type {
            const TYPE: $crate::flags::DataType = $crate::flags::DataType::Object(Some(
                <$type as $crate::class::RegisteredClass>::CLASS_NAME,
            ));

            #[inline]
            fn from_zval(zval: &'a $crate::types::Zval) -> ::std::option::Option<Self> {
                <Self as $crate::convert::FromZendObject>::from_zend_object(zval.object()?).ok()
            }
        }

        impl<'a> $crate::convert::FromZvalMut<'a> for &'a mut $type {
            const TYPE: $crate::flags::DataType = $crate::flags::DataType::Object(Some(
                <$type as $crate::class::RegisteredClass>::CLASS_NAME,
            ));

            #[inline]
            fn from_zval_mut(zval: &'a mut $crate::types::Zval) -> ::std::option::Option<Self> {
                <Self as $crate::convert::FromZendObjectMut>::from_zend_object_mut(
                    zval.object_mut()?,
                )
                .ok()
            }
        }

        impl $crate::convert::IntoZendObject for $type {
            #[inline]
            fn into_zend_object(
                self,
            ) -> $crate::error::Result<$crate::boxed::ZBox<$crate::types::ZendObject>> {
                Ok($crate::types::ZendClassObject::new(self).into())
            }
        }

        impl $crate::convert::IntoZval for $type {
            const TYPE: $crate::flags::DataType = $crate::flags::DataType::Object(Some(
                <$type as $crate::class::RegisteredClass>::CLASS_NAME,
            ));
            const NULLABLE: bool = false;

            #[inline]
            fn set_zval(
                self,
                zv: &mut $crate::types::Zval,
                persistent: bool,
            ) -> $crate::error::Result<()> {
                use $crate::convert::IntoZendObject;

                self.into_zend_object()?.set_zval(zv, persistent)
            }
        }
    };
}

/// Derives additional traits for cloneable [`RegisteredClass`] types to enable
/// using them as properties of other `#[php_class]` structs.
///
/// # Prefer `#[derive(PhpClone)]`
///
/// This macro is superseded by [`#[derive(PhpClone)]`](crate::PhpClone).
/// The derive macro is more ergonomic and follows Rust conventions:
///
/// ```ignore
/// use ext_php_rs::prelude::*;
///
/// #[php_class]
/// #[derive(Clone, PhpClone)]  // Preferred approach
/// struct Bar {
///     #[php(prop)]
///     value: String,
/// }
/// ```
///
/// This macro is kept for backward compatibility.
///
/// ---
///
/// This macro should be called for any `#[php_class]` struct that:
/// 1. Implements [`Clone`]
/// 2. Needs to be used as a property in another `#[php_class]` struct
///
/// The macro implements [`FromZendObject`] and [`FromZval`] for the owned type,
/// allowing PHP objects to be cloned into Rust values.
///
/// # Important: Clone Semantics
///
/// This macro creates a **clone** of the PHP object's underlying Rust data when
/// reading the property. This has important implications:
///
/// - **Reading** the property returns a cloned copy of the data
/// - **Writing** to the cloned object will NOT modify the original PHP object
/// - Each read creates a new independent clone
///
/// If you need to modify the original object, you should use methods on the
/// parent class that directly access the inner object, rather than reading
/// the property and modifying the clone.
///
/// # Rc/Arc Considerations
///
/// If your type contains [`Rc`], [`Arc`], or other reference-counted smart
/// pointers, be aware that cloning will create a new handle that shares the
/// underlying data with the original. This means:
///
/// - Mutations through the shared reference WILL affect both the original and clone
/// - The reference count will be incremented
/// - This may lead to unexpected shared state between PHP objects
///
/// Consider using deep cloning strategies if you need complete isolation.
///
/// [`Rc`]: std::rc::Rc
/// [`Arc`]: std::sync::Arc
///
/// # Example
///
/// ```ignore
/// use ext_php_rs::prelude::*;
/// use ext_php_rs::class_derives_clone;
///
/// #[php_class]
/// #[derive(Clone)]
/// struct Bar {
///     #[php(prop)]
///     value: String,
/// }
///
/// class_derives_clone!(Bar);
///
/// #[php_class]
/// struct Foo {
///     #[php(prop)]
///     bar: Bar, // Now works because Bar implements FromZval
/// }
/// ```
///
/// PHP usage demonstrating clone semantics:
/// ```php
/// $bar = new Bar("original");
/// $foo = new Foo($bar);
///
/// // Reading $foo->bar returns a clone
/// $barCopy = $foo->bar;
/// $barCopy->value = "modified";
///
/// // Original is unchanged because $barCopy is a clone
/// echo $foo->bar->value; // Outputs: "original"
/// ```
///
/// See: <https://github.com/extphprs/ext-php-rs/issues/182>
///
/// [`RegisteredClass`]: crate::class::RegisteredClass
/// [`FromZendObject`]: crate::convert::FromZendObject
/// [`FromZval`]: crate::convert::FromZval
#[macro_export]
macro_rules! class_derives_clone {
    ($type: ty) => {
        impl $crate::convert::FromZendObject<'_> for $type {
            fn from_zend_object(obj: &$crate::types::ZendObject) -> $crate::error::Result<Self> {
                let class_obj = $crate::types::ZendClassObject::<$type>::from_zend_obj(obj)
                    .ok_or($crate::error::Error::ZendClassObjectExtraction)?;
                Ok((**class_obj).clone())
            }
        }

        impl $crate::convert::FromZval<'_> for $type {
            const TYPE: $crate::flags::DataType = $crate::flags::DataType::Object(Some(
                <$type as $crate::class::RegisteredClass>::CLASS_NAME,
            ));

            fn from_zval(zval: &$crate::types::Zval) -> ::std::option::Option<Self> {
                let obj = zval.object()?;
                <Self as $crate::convert::FromZendObject>::from_zend_object(obj).ok()
            }
        }
    };
}

/// Derives `From<T> for Zval` and `IntoZval` for a given type.
macro_rules! into_zval {
    ($type: ty, $fn: ident, $dt: ident) => {
        impl From<$type> for $crate::types::Zval {
            fn from(val: $type) -> Self {
                let mut zv = Self::new();
                zv.$fn(val);
                zv
            }
        }

        impl $crate::convert::IntoZval for $type {
            const TYPE: $crate::flags::DataType = $crate::flags::DataType::$dt;
            const NULLABLE: bool = false;

            fn set_zval(self, zv: &mut $crate::types::Zval, _: bool) -> $crate::error::Result<()> {
                zv.$fn(self);
                Ok(())
            }
        }
    };
}

/// Derives `TryFrom<Zval> for T` and `FromZval for T` on a given type.
macro_rules! try_from_zval {
    ($type: ty, $fn: ident, $dt: ident) => {
        impl $crate::convert::FromZval<'_> for $type {
            const TYPE: $crate::flags::DataType = $crate::flags::DataType::$dt;

            fn from_zval(zval: &$crate::types::Zval) -> ::std::option::Option<Self> {
                use ::std::convert::TryInto;

                zval.$fn().and_then(|val| val.try_into().ok())
            }
        }

        impl ::std::convert::TryFrom<$crate::types::Zval> for $type {
            type Error = $crate::error::Error;

            fn try_from(value: $crate::types::Zval) -> $crate::error::Result<Self> {
                <Self as $crate::convert::FromZval>::from_zval(&value)
                    .ok_or($crate::error::Error::ZvalConversion(value.get_type()))
            }
        }
    };
}

/// Prints to the PHP standard output, without a newline.
///
/// Acts exactly the same as the built-in [`print`] macro.
///
/// # Panics
///
/// Panics if the generated string could not be converted to a `CString` due to
/// `NUL` characters.
#[macro_export]
macro_rules! php_print {
    ($arg: tt) => {{
        $crate::zend::printf($arg).expect("Failed to print to PHP stdout");
    }};

    ($($arg: tt) *) => {{
        let args = format!($($arg)*);
        $crate::zend::printf(args.as_str()).expect("Failed to print to PHP stdout");
    }};
}

/// Prints to the PHP standard output, with a newline.
///
/// The newline is only a newline character regardless of platform (no carriage
/// return).
///
/// Acts exactly the same as the built-in [`println`] macro.
///
/// # Panics
///
/// Panics if the generated string could not be converted to a `CString` due to
/// `NUL` characters.
#[macro_export]
macro_rules! php_println {
    () => {
        $crate::php_print!("\n");
    };

    ($fmt: tt) => {
        $crate::php_print!(concat!($fmt, "\n"));
    };

    ($fmt: tt, $($arg: tt) *) => {
        $crate::php_print!(concat!($fmt, "\n"), $($arg)*);
    };
}

/// Writes binary data to the PHP standard output.
///
/// Unlike [`php_print!`], this macro is binary-safe and can handle data
/// containing `NUL` bytes. It uses the SAPI module's `ub_write` function.
///
/// # Arguments
///
/// * `$data` - A byte slice (`&[u8]`) or byte literal (`b"..."`) to write.
///
/// # Returns
///
/// A `Result<usize>` containing the number of bytes written.
///
/// # Errors
///
/// Returns [`Error::SapiWriteUnavailable`] if the SAPI's `ub_write` function
/// is not available.
///
/// [`Error::SapiWriteUnavailable`]: crate::error::Error::SapiWriteUnavailable
///
/// # Examples
///
/// ```ignore
/// use ext_php_rs::php_write;
///
/// // Write a byte literal
/// php_write!(b"Hello World").expect("write failed");
///
/// // Write binary data with NUL bytes (would panic with php_print!)
/// php_write!(b"Hello\x00World").expect("write failed");
///
/// // Write a byte slice
/// let data: &[u8] = &[0x48, 0x65, 0x6c, 0x6c, 0x6f];
/// php_write!(data).expect("write failed");
/// ```
#[macro_export]
macro_rules! php_write {
    ($data: expr) => {{ $crate::zend::write($data) }};
}

/// Writes binary data to PHP's output stream with output buffering support.
///
/// This macro is both binary-safe (can handle `NUL` bytes) AND respects PHP's
/// output buffering (`ob_start()`). Use this when you need both capabilities.
///
/// # Arguments
///
/// * `$data` - A byte slice (`&[u8]`) or byte literal (`b"..."`) to write.
///
/// # Returns
///
/// The number of bytes written.
///
/// # Comparison
///
/// | Macro | Binary-safe | Output Buffering |
/// |-------|-------------|------------------|
/// | `php_print!` | No | Yes |
/// | `php_write!` | Yes | No (unbuffered) |
/// | `php_output!` | Yes | Yes |
///
/// # Examples
///
/// ```ignore
/// use ext_php_rs::php_output;
///
/// // Write binary data that will be captured by ob_start()
/// php_output!(b"Hello\x00World");
///
/// // Use with output buffering
/// // ob_start();
/// // php_output!(b"captured");
/// // $data = ob_get_clean(); // Contains "captured"
/// ```
#[macro_export]
macro_rules! php_output {
    ($data: expr) => {{ $crate::zend::output_write($data) }};
}
