#[macro_use]
extern crate ext_php_rs_derive;
/// Doc comments for MyClass.
/// This is a basic class example.
pub struct MyClass {}
impl ::ext_php_rs::class::RegisteredClass for MyClass {
    const CLASS_NAME: &'static str = "MyClass";
    const BUILDER_MODIFIER: ::std::option::Option<
        fn(::ext_php_rs::builders::ClassBuilder) -> ::ext_php_rs::builders::ClassBuilder,
    > = ::std::option::Option::None;
    const EXTENDS: ::std::option::Option<::ext_php_rs::class::ClassEntryInfo> = None;
    const IMPLEMENTS: &'static [::ext_php_rs::class::ClassEntryInfo] = &[];
    const FLAGS: ::ext_php_rs::flags::ClassFlags = ::ext_php_rs::flags::ClassFlags::empty();
    const DOC_COMMENTS: &'static [&'static str] = &[
        " Doc comments for MyClass.",
        " This is a basic class example.",
    ];
    #[inline]
    fn get_metadata() -> &'static ::ext_php_rs::class::ClassMetadata<Self> {
        static FIELD_PROPS: [::ext_php_rs::internal::property::PropertyDescriptor<
            MyClass,
        >; 0usize] = [];
        static METADATA: ::ext_php_rs::class::ClassMetadata<MyClass> = ::ext_php_rs::class::ClassMetadata::new(
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
}
impl MyClass {
    pub fn get_first(&self) -> i64 {
        1
    }
    pub fn set_first(&mut self, _value: i64) {}
    pub fn get_second(&self) -> String {
        String::new()
    }
    pub fn plain(&self) {}
}
impl ::ext_php_rs::internal::class::PhpClassImpl<MyClass>
for ::ext_php_rs::internal::class::PhpClassImplCollector<MyClass> {
    fn get_methods(
        self,
    ) -> ::std::vec::Vec<
        (
            ::ext_php_rs::builders::FunctionBuilder<'static>,
            ::ext_php_rs::flags::MethodFlags,
        ),
    > {
        ::alloc::boxed::box_assume_init_into_vec_unsafe(
            ::alloc::intrinsics::write_box_via_move(
                ::alloc::boxed::Box::new_uninit(),
                [
                    (
                        {
                            const __REQUIRED: usize = ::ext_php_rs::args::required_count(
                                &[],
                            );
                            ::ext_php_rs::builders::FunctionBuilder::new(
                                    "plain",
                                    {
                                        (/*ERROR*/);
                                        handler
                                    },
                                )
                                .required_args(__REQUIRED)
                                .returns(::ext_php_rs::flags::DataType::Void, false, false)
                        },
                        ::ext_php_rs::flags::MethodFlags::Public,
                    ),
                ],
            ),
        )
    }
    fn get_method_props(
        self,
    ) -> &'static [::ext_php_rs::internal::property::PropertyDescriptor<MyClass>] {
        fn __method_get_0(
            this: &MyClass,
            __zv: &mut ::ext_php_rs::types::Zval,
        ) -> ::ext_php_rs::exception::PhpResult {
            use ::ext_php_rs::convert::IntoZval as _;
            let value = MyClass::get_first(this);
            value
                .set_zval(__zv, false)
                .map_err(|e| ::alloc::__export::must_use({
                    ::alloc::fmt::format(
                        format_args!("Failed to return property value: {0:?}", e),
                    )
                }))?;
            Ok(())
        }
        fn __method_set_0(
            this: &mut MyClass,
            __zv: &::ext_php_rs::types::Zval,
        ) -> ::ext_php_rs::exception::PhpResult {
            use ::ext_php_rs::convert::FromZval as _;
            let val = <i64 as ::ext_php_rs::convert::FromZval>::from_zval(__zv)
                .ok_or("Unable to convert property value into required type.")?;
            MyClass::set_first(this, val);
            Ok(())
        }
        fn __method_get_1(
            this: &MyClass,
            __zv: &mut ::ext_php_rs::types::Zval,
        ) -> ::ext_php_rs::exception::PhpResult {
            use ::ext_php_rs::convert::IntoZval as _;
            let value = MyClass::get_second(this);
            value
                .set_zval(__zv, false)
                .map_err(|e| ::alloc::__export::must_use({
                    ::alloc::fmt::format(
                        format_args!("Failed to return property value: {0:?}", e),
                    )
                }))?;
            Ok(())
        }
        static METHOD_PROPS: [::ext_php_rs::internal::property::PropertyDescriptor<
            MyClass,
        >; 2usize] = [
            ::ext_php_rs::internal::property::PropertyDescriptor {
                name: "first",
                get: ::std::option::Option::Some(__method_get_0),
                set: ::std::option::Option::Some(__method_set_0),
                flags: ::ext_php_rs::flags::PropertyFlags::Public,
                docs: &[],
                ty: <i64 as ::ext_php_rs::convert::IntoZval>::TYPE,
                nullable: <i64 as ::ext_php_rs::convert::IntoZval>::NULLABLE,
                readonly: false,
            },
            ::ext_php_rs::internal::property::PropertyDescriptor {
                name: "second",
                get: ::std::option::Option::Some(__method_get_1),
                set: ::std::option::Option::None,
                flags: ::ext_php_rs::flags::PropertyFlags::Public,
                docs: &[],
                ty: <String as ::ext_php_rs::convert::IntoZval>::TYPE,
                nullable: <String as ::ext_php_rs::convert::IntoZval>::NULLABLE,
                readonly: true,
            },
        ];
        &METHOD_PROPS
    }
    fn get_constructor(
        self,
    ) -> ::std::option::Option<::ext_php_rs::class::ConstructorMeta<MyClass>> {
        ::std::option::Option::None
    }
    fn get_constants(
        self,
    ) -> &'static [(
        &'static str,
        &'static dyn ::ext_php_rs::convert::IntoZvalDyn,
        &'static [&'static str],
        ::ext_php_rs::flags::ConstantFlags,
    )] {
        &[]
    }
}
