use std::{any::TypeId, marker::PhantomData};

use crate::{
    builders::FunctionBuilder,
    class::{ClassEntryInfo, ConstructorMeta, RegisteredClass},
    convert::{IntoZval, IntoZvalDyn},
    describe::DocComments,
    flags::{ConstantFlags, MethodFlags},
    internal::property::PropertyDescriptor,
};

/// Registration entry for interface implementations.
/// Used by `#[php_impl_interface]` macro to register interfaces across crate boundaries.
pub struct InterfaceRegistration {
    /// The `TypeId` of the class implementing the interface.
    pub class_type_id: TypeId,
    /// Function that returns the interface's `ClassEntryInfo`.
    pub interface_getter: fn() -> ClassEntryInfo,
}

inventory::collect!(InterfaceRegistration);

/// Trait for getting interface method builders.
/// This trait uses autoref specialization to allow optional implementation.
/// Classes with `#[php_impl_interface]` will implement this directly (not on a reference).
pub trait InterfaceMethodsProvider<T: RegisteredClass> {
    fn get_interface_methods(self) -> Vec<(FunctionBuilder<'static>, MethodFlags)>;
}

/// Default implementation for classes without interface implementations.
/// Uses autoref specialization - the reference implementation is chosen when
/// no direct implementation exists.
impl<T: RegisteredClass> InterfaceMethodsProvider<T> for &'_ PhpClassImplCollector<T> {
    #[inline]
    fn get_interface_methods(self) -> Vec<(FunctionBuilder<'static>, MethodFlags)> {
        Vec::new()
    }
}

/// Collector used to collect methods for PHP classes.
pub struct PhpClassImplCollector<T: RegisteredClass>(PhantomData<T>);

impl<T: RegisteredClass> Default for PhpClassImplCollector<T> {
    #[inline]
    fn default() -> Self {
        Self(PhantomData)
    }
}

pub trait PhpClassImpl<T: RegisteredClass> {
    fn get_methods(self) -> Vec<(FunctionBuilder<'static>, MethodFlags)>;
    fn get_method_props(self) -> &'static [PropertyDescriptor<T>];
    fn get_constructor(self) -> Option<ConstructorMeta<T>>;
    fn get_constants(
        self,
    ) -> &'static [(
        &'static str,
        &'static dyn IntoZvalDyn,
        DocComments,
        ConstantFlags,
    )];
}

/// Default implementation for classes without an `impl` block. Classes that do
/// have an `impl` block will override this by implementing `PhpClassImpl` for
/// `PhpClassImplCollector<ClassName>` (note the missing reference). This is
/// `dtolnay` specialisation: <https://github.com/dtolnay/case-studies/blob/master/autoref-specialization/README.md>
impl<T: RegisteredClass> PhpClassImpl<T> for &'_ PhpClassImplCollector<T> {
    #[inline]
    fn get_methods(self) -> Vec<(FunctionBuilder<'static>, MethodFlags)> {
        Vec::default()
    }

    #[inline]
    fn get_method_props(self) -> &'static [PropertyDescriptor<T>] {
        &[]
    }

    #[inline]
    fn get_constructor(self) -> Option<ConstructorMeta<T>> {
        Option::default()
    }

    #[inline]
    fn get_constants(
        self,
    ) -> &'static [(
        &'static str,
        &'static dyn IntoZvalDyn,
        DocComments,
        ConstantFlags,
    )] {
        &[]
    }
}

/// Probe for an optional `Default` implementation on a registered class.
///
/// Autoref specialisation: the by-value impl needs `T: Default`, the
/// reference impl is the fallback, so `DefaultProbe::<T>::default().default_init()`
/// resolves at compile time without the macro reading derive attributes.
pub struct DefaultProbe<T>(PhantomData<T>);

impl<T> Default for DefaultProbe<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

pub trait ProbeDefault<T> {
    fn default_init(self) -> Option<T>;
}

impl<T: Default> ProbeDefault<T> for DefaultProbe<T> {
    #[inline]
    fn default_init(self) -> Option<T> {
        Some(T::default())
    }
}

impl<T> ProbeDefault<T> for &'_ DefaultProbe<T> {
    #[inline]
    fn default_init(self) -> Option<T> {
        None
    }
}

/// Probe for an optional `Clone` implementation on a registered class, same
/// mechanism as [`DefaultProbe`].
pub struct CloneProbe<T>(PhantomData<T>);

impl<T> Default for CloneProbe<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

pub trait ProbeClone<T> {
    fn clone_obj(self, value: &T) -> Option<T>;
}

impl<T: Clone> ProbeClone<T> for CloneProbe<T> {
    #[inline]
    fn clone_obj(self, value: &T) -> Option<T> {
        Some(value.clone())
    }
}

impl<T> ProbeClone<T> for &'_ CloneProbe<T> {
    #[inline]
    fn clone_obj(self, _: &T) -> Option<T> {
        None
    }
}

// This implementation is only used for `TYPE` and `NULLABLE`.
impl<T: RegisteredClass + IntoZval> IntoZval for PhpClassImplCollector<T> {
    const TYPE: crate::flags::DataType = T::TYPE;
    const NULLABLE: bool = T::NULLABLE;

    #[inline]
    fn set_zval(self, _: &mut crate::types::Zval, _: bool) -> crate::error::Result<()> {
        unreachable!();
    }
}
