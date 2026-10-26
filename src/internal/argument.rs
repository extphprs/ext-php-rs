//! Conversion of the PHP arguments of the functions and methods that the
//! macros generate.
//!
//! Each argument goes through [`FunctionArgument`]. A `#[php_class]` parameter
//! written `&T` or `&mut T` borrows the Rust value of the object through a
//! guard. The generated function keeps the guard in a [`ArgumentHolder`] for
//! the whole call, so PHP code that the call runs cannot modify the value.

use crate::{
    args::Arg,
    class::RegisteredClass,
    convert::FromZvalMut,
    error::{Error, Result},
    exception::PhpException,
    flags::DataType,
    types::{ClassRef, ClassRefMut, ZendClassObject, Zval},
    zend::ce,
};

mod sealed {
    pub trait Sealed {}

    impl Sealed for () {}

    impl<T> Sealed for Option<T> {}
}

/// Storage for the guard of one argument, declared by the generated function
/// before it converts the argument.
pub trait ArgumentHolder: Sized + sealed::Sealed {
    /// The empty holder.
    const INIT: Self;
}

impl ArgumentHolder for () {
    const INIT: Self = ();
}

impl<T> ArgumentHolder for Option<T> {
    const INIT: Self = None;
}

/// A type that a generated function can take as a parameter.
///
/// Every [`FromZvalMut`] type implements it with `FROM_ZVAL = true` and an
/// empty holder. `&T`, `&mut T`, `Option<&T>` and `Option<&mut T>` of a
/// `#[php_class]` type implement it with `FROM_ZVAL = false`: the holder keeps
/// the guard of the borrow. The two values of `FROM_ZVAL` keep the
/// implementations apart, and the compiler finds the one that applies.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used as a PHP function argument",
    label = "no `FromZval` or `FromZvalMut` implementation for `{Self}`",
    note = "a `#[php_class]` parameter is `&T`, `&mut T`, `Option<&T>`, `Option<&mut T>`, `ClassRef<T>` or `ClassRefMut<T>`",
    note = "a variadic parameter must be spelled `&[T]` in the signature, a type alias hiding the slice is not detected"
)]
pub trait FunctionArgument<'a, 'h, const FROM_ZVAL: bool>: Sized {
    /// The storage that the generated function keeps for the whole call.
    type Holder: ArgumentHolder;

    /// The PHP type of the parameter.
    const TYPE: DataType;

    /// Whether the parameter accepts `null` and may be omitted.
    const NULLABLE: bool;

    /// Whether the parameter is declared pass-by-reference.
    const BY_REF: bool;

    /// Converts the argument.
    ///
    /// # Errors
    ///
    /// * [`Error::ZvalConversion`] - The value is not a `Self`.
    /// * [`Error::Borrow`] - The value is a class object whose Rust value is
    ///   borrowed or not initialized.
    fn extract(zval: &'a mut Zval, holder: &'h mut Self::Holder) -> Result<Self>;

    /// The value of the argument when the caller omitted it, `None` when the
    /// argument is required.
    fn missing() -> Option<Self>;
}

impl<'a, 'h, T: FromZvalMut<'a>> FunctionArgument<'a, 'h, true> for T {
    type Holder = ();

    const TYPE: DataType = T::TYPE;
    const NULLABLE: bool = T::NULLABLE;
    const BY_REF: bool = T::BY_REF;

    #[inline]
    fn extract(zval: &'a mut Zval, &mut (): &'h mut ()) -> Result<Self> {
        T::try_from_zval_mut(zval)
    }

    #[inline]
    fn missing() -> Option<Self> {
        T::from_missing()
    }
}

impl<'a: 'h, 'h, T: RegisteredClass> FunctionArgument<'a, 'h, false> for &'h T {
    type Holder = Option<ClassRef<'a, T>>;

    const TYPE: DataType = DataType::object(T::CLASS_NAME);
    const NULLABLE: bool = false;
    const BY_REF: bool = false;

    #[inline]
    fn extract(zval: &'a mut Zval, holder: &'h mut Self::Holder) -> Result<Self> {
        let guard = ZendClassObject::<T>::from_zval_uninit(zval)?.try_borrow()?;
        Ok(&**holder.insert(guard))
    }

    #[inline]
    fn missing() -> Option<Self> {
        None
    }
}

impl<'a: 'h, 'h, T: RegisteredClass> FunctionArgument<'a, 'h, false> for &'h mut T {
    type Holder = Option<ClassRefMut<'a, T>>;

    const TYPE: DataType = DataType::object(T::CLASS_NAME);
    const NULLABLE: bool = false;
    const BY_REF: bool = false;

    #[inline]
    fn extract(zval: &'a mut Zval, holder: &'h mut Self::Holder) -> Result<Self> {
        let guard = ZendClassObject::<T>::from_zval_uninit(zval)?.try_borrow_mut()?;
        Ok(&mut **holder.insert(guard))
    }

    #[inline]
    fn missing() -> Option<Self> {
        None
    }
}

impl<'a: 'h, 'h, T: RegisteredClass> FunctionArgument<'a, 'h, false> for Option<&'h T> {
    type Holder = Option<ClassRef<'a, T>>;

    const TYPE: DataType = DataType::object(T::CLASS_NAME);
    const NULLABLE: bool = true;
    const BY_REF: bool = false;

    #[inline]
    fn extract(zval: &'a mut Zval, holder: &'h mut Self::Holder) -> Result<Self> {
        if zval.is_null() {
            return Ok(None);
        }
        <&T as FunctionArgument<'a, 'h, false>>::extract(zval, holder).map(Some)
    }

    #[inline]
    fn missing() -> Option<Self> {
        Some(None)
    }
}

impl<'a: 'h, 'h, T: RegisteredClass> FunctionArgument<'a, 'h, false> for Option<&'h mut T> {
    type Holder = Option<ClassRefMut<'a, T>>;

    const TYPE: DataType = DataType::object(T::CLASS_NAME);
    const NULLABLE: bool = true;
    const BY_REF: bool = false;

    #[inline]
    fn extract(zval: &'a mut Zval, holder: &'h mut Self::Holder) -> Result<Self> {
        if zval.is_null() {
            return Ok(None);
        }
        <&mut T as FunctionArgument<'a, 'h, false>>::extract(zval, holder).map(Some)
    }

    #[inline]
    fn missing() -> Option<Self> {
        Some(None)
    }
}

/// The empty holder of an argument of type `T`.
#[inline]
#[must_use]
pub fn holder<'a, 'h, T, const FROM_ZVAL: bool>() -> T::Holder
where
    T: FunctionArgument<'a, 'h, FROM_ZVAL>,
{
    <T::Holder as ArgumentHolder>::INIT
}

/// Converts an argument of type `T`.
///
/// # Errors
///
/// See [`FunctionArgument::extract`].
#[inline]
pub fn extract<'a, 'h, T, const FROM_ZVAL: bool>(
    zval: &'a mut Zval,
    holder: &'h mut T::Holder,
) -> Result<T>
where
    T: FunctionArgument<'a, 'h, FROM_ZVAL>,
{
    T::extract(zval, holder)
}

/// The value of an omitted argument of type `T`, `None` when it is required.
#[inline]
#[must_use]
pub fn missing<'a, 'h, T, const FROM_ZVAL: bool>() -> Option<T>
where
    T: FunctionArgument<'a, 'h, FROM_ZVAL>,
{
    T::missing()
}

/// Whether an argument of type `T` accepts `null` and may be omitted.
#[must_use]
pub const fn nullable<'a, 'h, T, const FROM_ZVAL: bool>() -> bool
where
    T: FunctionArgument<'a, 'h, FROM_ZVAL>,
{
    T::NULLABLE
}

/// The [`Arg`] of a parameter of type `T`.
pub fn arg<'z, 'a, 'h, T, const FROM_ZVAL: bool>(name: impl Into<String>) -> Arg<'z>
where
    T: FunctionArgument<'a, 'h, FROM_ZVAL>,
{
    Arg::typed(name, T::TYPE, T::NULLABLE, T::BY_REF)
}

/// The exception for an argument that failed to convert: the `Error` of a
/// [`BorrowError`](crate::types::BorrowError), or a `TypeError` for a value of
/// the wrong type.
#[must_use]
pub fn argument_error(name: &str, error: &Error) -> PhpException {
    match error {
        Error::Borrow(borrow) => (*borrow).into(),
        _ => PhpException::new(
            format!("Invalid value given for argument `{name}`."),
            0,
            ce::type_error(),
        ),
    }
}
