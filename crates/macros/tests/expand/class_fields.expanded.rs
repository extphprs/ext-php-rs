#[macro_use]
extern crate ext_php_rs_derive;
use ext_php_rs::types::Zval;
pub struct Emitter {
    listeners: Vec<Zval>,
    count: i64,
}
impl ::ext_php_rs::class::RegisteredClass for Emitter {
    const CLASS_NAME: &'static str = "Emitter";
    const BUILDER_MODIFIER: ::std::option::Option<
        fn(::ext_php_rs::builders::ClassBuilder) -> ::ext_php_rs::builders::ClassBuilder,
    > = ::std::option::Option::None;
    const EXTENDS: ::std::option::Option<::ext_php_rs::class::ClassEntryInfo> = None;
    const IMPLEMENTS: &'static [::ext_php_rs::class::ClassEntryInfo] = &[];
    const FLAGS: ::ext_php_rs::flags::ClassFlags = ::ext_php_rs::flags::ClassFlags::empty();
    const DOC_COMMENTS: &'static [&'static str] = &[];
    #[inline]
    fn get_metadata() -> &'static ::ext_php_rs::class::ClassMetadata<Self> {
        static FIELD_PROPS: [::ext_php_rs::internal::property::PropertyDescriptor<
            Emitter,
        >; 0usize] = [];
        static METADATA: ::ext_php_rs::class::ClassMetadata<Emitter> = ::ext_php_rs::class::ClassMetadata::new(
            &FIELD_PROPS,
        );
        &METADATA
    }
    #[must_use]
    fn static_properties() -> &'static [(
        &'static str,
        ::ext_php_rs::flags::PropertyFlags,
        ::std::option::Option<&'static (dyn ::ext_php_rs::convert::IntoZvalDyn + Sync)>,
        &'static [&'static str],
    )] {
        static STATIC_PROPS: &[(
            &str,
            ::ext_php_rs::flags::PropertyFlags,
            ::std::option::Option<
                &'static (dyn ::ext_php_rs::convert::IntoZvalDyn + Sync),
            >,
            &[&str],
        )] = &[];
        STATIC_PROPS
    }
    #[inline]
    fn method_properties() -> &'static [::ext_php_rs::internal::property::PropertyDescriptor<
        Self,
    >] {
        use ::ext_php_rs::internal::class::PhpClassImpl;
        ::ext_php_rs::internal::class::PhpClassImplCollector::<Self>::default()
            .get_method_props()
    }
    #[inline]
    fn method_builders() -> ::std::vec::Vec<
        (
            ::ext_php_rs::builders::FunctionBuilder<'static>,
            ::ext_php_rs::flags::MethodFlags,
        ),
    > {
        use ::ext_php_rs::internal::class::PhpClassImpl;
        ::ext_php_rs::internal::class::PhpClassImplCollector::<Self>::default()
            .get_methods()
    }
    #[inline]
    fn constructor() -> ::std::option::Option<
        ::ext_php_rs::class::ConstructorMeta<Self>,
    > {
        use ::ext_php_rs::internal::class::PhpClassImpl;
        ::ext_php_rs::internal::class::PhpClassImplCollector::<Self>::default()
            .get_constructor()
    }
    #[inline]
    fn constants() -> &'static [(
        &'static str,
        &'static dyn ::ext_php_rs::convert::IntoZvalDyn,
        &'static [&'static str],
        ::ext_php_rs::flags::ConstantFlags,
    )] {
        use ::ext_php_rs::internal::class::PhpClassImpl;
        ::ext_php_rs::internal::class::PhpClassImplCollector::<Self>::default()
            .get_constants()
    }
    #[inline]
    fn interface_implementations() -> ::std::vec::Vec<
        ::ext_php_rs::class::ClassEntryInfo,
    > {
        let my_type_id = ::std::any::TypeId::of::<Self>();
        ::ext_php_rs::inventory::iter::<
            ::ext_php_rs::internal::class::InterfaceRegistration,
        >()
            .filter(|reg| reg.class_type_id == my_type_id)
            .map(|reg| (reg.interface_getter)())
            .collect()
    }
    #[inline]
    fn interface_method_implementations() -> ::std::vec::Vec<
        (
            ::ext_php_rs::builders::FunctionBuilder<'static>,
            ::ext_php_rs::flags::MethodFlags,
        ),
    > {
        use ::ext_php_rs::internal::class::InterfaceMethodsProvider;
        ::ext_php_rs::internal::class::PhpClassImplCollector::<Self>::default()
            .get_interface_methods()
    }
    #[inline]
    #[must_use]
    fn default_init() -> ::std::option::Option<Self> {
        use ::ext_php_rs::internal::class::ProbeDefault as _;
        ::ext_php_rs::internal::class::DefaultProbe::<Self>::default().default_init()
    }
    #[inline]
    #[must_use]
    fn clone_obj(&self) -> ::std::option::Option<Self> {
        use ::ext_php_rs::internal::class::ProbeClone as _;
        ::ext_php_rs::internal::class::CloneProbe::<Self>::default().clone_obj(self)
    }
    #[inline]
    fn gc_trace(&self, roots: &mut ::ext_php_rs::gc::GcRoots) {
        use ::ext_php_rs::internal::class::ProbeGcTrace as _;
        unsafe {
            ::ext_php_rs::internal::class::GcTraceProbe::<Vec<Zval>>::default()
                .trace_field(&self.listeners, roots);
            ::ext_php_rs::internal::class::GcTraceProbe::<i64>::default()
                .trace_field(&self.count, roots);
        }
    }
}
impl<'a> ::ext_php_rs::convert::FromZendObject<'a> for &'a Emitter {
    #[inline]
    fn from_zend_object(
        obj: &'a ::ext_php_rs::types::ZendObject,
    ) -> ::ext_php_rs::error::Result<Self> {
        let obj = ::ext_php_rs::types::ZendClassObject::<Emitter>::from_zend_obj(obj)
            .ok_or(::ext_php_rs::error::Error::InvalidScope)?;
        Ok(&**obj)
    }
}
impl<'a> ::ext_php_rs::convert::FromZendObjectMut<'a> for &'a mut Emitter {
    #[inline]
    fn from_zend_object_mut(
        obj: &'a mut ::ext_php_rs::types::ZendObject,
    ) -> ::ext_php_rs::error::Result<Self> {
        let obj = ::ext_php_rs::types::ZendClassObject::<Emitter>::from_zend_obj_mut(obj)
            .ok_or(::ext_php_rs::error::Error::InvalidScope)?;
        Ok(&mut **obj)
    }
}
impl<'a> ::ext_php_rs::convert::FromZval<'a> for &'a Emitter {
    const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::object(
        <Emitter as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
    );
    #[inline]
    fn from_zval(zval: &'a ::ext_php_rs::types::Zval) -> ::std::option::Option<Self> {
        <Self as ::ext_php_rs::convert::FromZendObject>::from_zend_object(zval.object()?)
            .ok()
    }
}
impl<'a> ::ext_php_rs::convert::FromZvalMut<'a> for &'a mut Emitter {
    const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::object(
        <Emitter as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
    );
    #[inline]
    fn from_zval_mut(
        zval: &'a mut ::ext_php_rs::types::Zval,
    ) -> ::std::option::Option<Self> {
        <Self as ::ext_php_rs::convert::FromZendObjectMut>::from_zend_object_mut(
                zval.object_mut()?,
            )
            .ok()
    }
}
impl ::ext_php_rs::convert::IntoZendObject for Emitter {
    #[inline]
    fn into_zend_object(
        self,
    ) -> ::ext_php_rs::error::Result<
        ::ext_php_rs::boxed::ZBox<::ext_php_rs::types::ZendObject>,
    > {
        Ok(::ext_php_rs::types::ZendClassObject::new(self).into())
    }
}
impl ::ext_php_rs::convert::IntoZval for Emitter {
    const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::object(
        <Emitter as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
    );
    const NULLABLE: bool = false;
    #[inline]
    fn set_zval(
        self,
        zv: &mut ::ext_php_rs::types::Zval,
        persistent: bool,
    ) -> ::ext_php_rs::error::Result<()> {
        use ::ext_php_rs::convert::IntoZendObject;
        self.into_zend_object()?.set_zval(zv, persistent)
    }
}
pub struct Pair(Zval, String);
impl ::ext_php_rs::class::RegisteredClass for Pair {
    const CLASS_NAME: &'static str = "Pair";
    const BUILDER_MODIFIER: ::std::option::Option<
        fn(::ext_php_rs::builders::ClassBuilder) -> ::ext_php_rs::builders::ClassBuilder,
    > = ::std::option::Option::None;
    const EXTENDS: ::std::option::Option<::ext_php_rs::class::ClassEntryInfo> = None;
    const IMPLEMENTS: &'static [::ext_php_rs::class::ClassEntryInfo] = &[];
    const FLAGS: ::ext_php_rs::flags::ClassFlags = ::ext_php_rs::flags::ClassFlags::empty();
    const DOC_COMMENTS: &'static [&'static str] = &[];
    #[inline]
    fn get_metadata() -> &'static ::ext_php_rs::class::ClassMetadata<Self> {
        static FIELD_PROPS: [::ext_php_rs::internal::property::PropertyDescriptor<
            Pair,
        >; 0usize] = [];
        static METADATA: ::ext_php_rs::class::ClassMetadata<Pair> = ::ext_php_rs::class::ClassMetadata::new(
            &FIELD_PROPS,
        );
        &METADATA
    }
    #[must_use]
    fn static_properties() -> &'static [(
        &'static str,
        ::ext_php_rs::flags::PropertyFlags,
        ::std::option::Option<&'static (dyn ::ext_php_rs::convert::IntoZvalDyn + Sync)>,
        &'static [&'static str],
    )] {
        static STATIC_PROPS: &[(
            &str,
            ::ext_php_rs::flags::PropertyFlags,
            ::std::option::Option<
                &'static (dyn ::ext_php_rs::convert::IntoZvalDyn + Sync),
            >,
            &[&str],
        )] = &[];
        STATIC_PROPS
    }
    #[inline]
    fn method_properties() -> &'static [::ext_php_rs::internal::property::PropertyDescriptor<
        Self,
    >] {
        use ::ext_php_rs::internal::class::PhpClassImpl;
        ::ext_php_rs::internal::class::PhpClassImplCollector::<Self>::default()
            .get_method_props()
    }
    #[inline]
    fn method_builders() -> ::std::vec::Vec<
        (
            ::ext_php_rs::builders::FunctionBuilder<'static>,
            ::ext_php_rs::flags::MethodFlags,
        ),
    > {
        use ::ext_php_rs::internal::class::PhpClassImpl;
        ::ext_php_rs::internal::class::PhpClassImplCollector::<Self>::default()
            .get_methods()
    }
    #[inline]
    fn constructor() -> ::std::option::Option<
        ::ext_php_rs::class::ConstructorMeta<Self>,
    > {
        use ::ext_php_rs::internal::class::PhpClassImpl;
        ::ext_php_rs::internal::class::PhpClassImplCollector::<Self>::default()
            .get_constructor()
    }
    #[inline]
    fn constants() -> &'static [(
        &'static str,
        &'static dyn ::ext_php_rs::convert::IntoZvalDyn,
        &'static [&'static str],
        ::ext_php_rs::flags::ConstantFlags,
    )] {
        use ::ext_php_rs::internal::class::PhpClassImpl;
        ::ext_php_rs::internal::class::PhpClassImplCollector::<Self>::default()
            .get_constants()
    }
    #[inline]
    fn interface_implementations() -> ::std::vec::Vec<
        ::ext_php_rs::class::ClassEntryInfo,
    > {
        let my_type_id = ::std::any::TypeId::of::<Self>();
        ::ext_php_rs::inventory::iter::<
            ::ext_php_rs::internal::class::InterfaceRegistration,
        >()
            .filter(|reg| reg.class_type_id == my_type_id)
            .map(|reg| (reg.interface_getter)())
            .collect()
    }
    #[inline]
    fn interface_method_implementations() -> ::std::vec::Vec<
        (
            ::ext_php_rs::builders::FunctionBuilder<'static>,
            ::ext_php_rs::flags::MethodFlags,
        ),
    > {
        use ::ext_php_rs::internal::class::InterfaceMethodsProvider;
        ::ext_php_rs::internal::class::PhpClassImplCollector::<Self>::default()
            .get_interface_methods()
    }
    #[inline]
    #[must_use]
    fn default_init() -> ::std::option::Option<Self> {
        use ::ext_php_rs::internal::class::ProbeDefault as _;
        ::ext_php_rs::internal::class::DefaultProbe::<Self>::default().default_init()
    }
    #[inline]
    #[must_use]
    fn clone_obj(&self) -> ::std::option::Option<Self> {
        use ::ext_php_rs::internal::class::ProbeClone as _;
        ::ext_php_rs::internal::class::CloneProbe::<Self>::default().clone_obj(self)
    }
    #[inline]
    fn gc_trace(&self, roots: &mut ::ext_php_rs::gc::GcRoots) {
        use ::ext_php_rs::internal::class::ProbeGcTrace as _;
        unsafe {
            ::ext_php_rs::internal::class::GcTraceProbe::<Zval>::default()
                .trace_field(&self.0, roots);
            ::ext_php_rs::internal::class::GcTraceProbe::<String>::default()
                .trace_field(&self.1, roots);
        }
    }
}
impl<'a> ::ext_php_rs::convert::FromZendObject<'a> for &'a Pair {
    #[inline]
    fn from_zend_object(
        obj: &'a ::ext_php_rs::types::ZendObject,
    ) -> ::ext_php_rs::error::Result<Self> {
        let obj = ::ext_php_rs::types::ZendClassObject::<Pair>::from_zend_obj(obj)
            .ok_or(::ext_php_rs::error::Error::InvalidScope)?;
        Ok(&**obj)
    }
}
impl<'a> ::ext_php_rs::convert::FromZendObjectMut<'a> for &'a mut Pair {
    #[inline]
    fn from_zend_object_mut(
        obj: &'a mut ::ext_php_rs::types::ZendObject,
    ) -> ::ext_php_rs::error::Result<Self> {
        let obj = ::ext_php_rs::types::ZendClassObject::<Pair>::from_zend_obj_mut(obj)
            .ok_or(::ext_php_rs::error::Error::InvalidScope)?;
        Ok(&mut **obj)
    }
}
impl<'a> ::ext_php_rs::convert::FromZval<'a> for &'a Pair {
    const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::object(
        <Pair as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
    );
    #[inline]
    fn from_zval(zval: &'a ::ext_php_rs::types::Zval) -> ::std::option::Option<Self> {
        <Self as ::ext_php_rs::convert::FromZendObject>::from_zend_object(zval.object()?)
            .ok()
    }
}
impl<'a> ::ext_php_rs::convert::FromZvalMut<'a> for &'a mut Pair {
    const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::object(
        <Pair as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
    );
    #[inline]
    fn from_zval_mut(
        zval: &'a mut ::ext_php_rs::types::Zval,
    ) -> ::std::option::Option<Self> {
        <Self as ::ext_php_rs::convert::FromZendObjectMut>::from_zend_object_mut(
                zval.object_mut()?,
            )
            .ok()
    }
}
impl ::ext_php_rs::convert::IntoZendObject for Pair {
    #[inline]
    fn into_zend_object(
        self,
    ) -> ::ext_php_rs::error::Result<
        ::ext_php_rs::boxed::ZBox<::ext_php_rs::types::ZendObject>,
    > {
        Ok(::ext_php_rs::types::ZendClassObject::new(self).into())
    }
}
impl ::ext_php_rs::convert::IntoZval for Pair {
    const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::object(
        <Pair as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
    );
    const NULLABLE: bool = false;
    #[inline]
    fn set_zval(
        self,
        zv: &mut ::ext_php_rs::types::Zval,
        persistent: bool,
    ) -> ::ext_php_rs::error::Result<()> {
        use ::ext_php_rs::convert::IntoZendObject;
        self.into_zend_object()?.set_zval(zv, persistent)
    }
}
