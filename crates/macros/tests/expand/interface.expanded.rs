#[macro_use]
extern crate ext_php_rs_derive;
/// Doc comments for MyInterface.
/// This is a basic interface example.
trait MyInterface {
    /// Doc comments for MY_CONST.
    const MY_CONST: i32 = 42;
    /// Doc comments for my_method.
    /// This method does something.
    fn my_method(&self, arg: i32) -> String;
}
pub struct PhpInterfaceMyInterface;
impl ::ext_php_rs::class::RegisteredClass for PhpInterfaceMyInterface {
    const CLASS_NAME: &'static str = "MyInterface";
    const BUILDER_MODIFIER: Option<
        fn(::ext_php_rs::builders::ClassBuilder) -> ::ext_php_rs::builders::ClassBuilder,
    > = None;
    const EXTENDS: Option<::ext_php_rs::class::ClassEntryInfo> = None;
    const FLAGS: ::ext_php_rs::flags::ClassFlags = ::ext_php_rs::flags::ClassFlags::Interface;
    const IMPLEMENTS: &'static [::ext_php_rs::class::ClassEntryInfo] = &[];
    const DOC_COMMENTS: &'static [&'static str] = &[
        " Doc comments for MyInterface.",
        " This is a basic interface example.",
    ];
    fn get_metadata() -> &'static ::ext_php_rs::class::ClassMetadata<Self> {
        static METADATA: ::ext_php_rs::class::ClassMetadata<PhpInterfaceMyInterface> = ::ext_php_rs::class::ClassMetadata::new(
            &[],
        );
        &METADATA
    }
    fn method_builders() -> Vec<
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
                                &[
                                    <i32 as ::ext_php_rs::convert::FromZvalMut>::NULLABLE
                                        || false,
                                ],
                            );
                            ::ext_php_rs::builders::FunctionBuilder::new_abstract(
                                    "myMethod",
                                )
                                .arg(::ext_php_rs::args::Arg::of::<i32>("arg"))
                                .required_args(__REQUIRED)
                                .returns(
                                    <String as ::ext_php_rs::convert::IntoZval>::TYPE,
                                    false,
                                    <String as ::ext_php_rs::convert::IntoZval>::NULLABLE,
                                )
                                .docs(
                                    &[
                                        " Doc comments for my_method.",
                                        " This method does something.",
                                    ],
                                )
                        },
                        ::ext_php_rs::flags::MethodFlags::Public
                            | ::ext_php_rs::flags::MethodFlags::Abstract,
                    ),
                ],
            ),
        )
    }
    fn constructor() -> Option<::ext_php_rs::class::ConstructorMeta<Self>> {
        None
    }
    fn constants() -> &'static [(
        &'static str,
        &'static dyn ext_php_rs::convert::IntoZvalDyn,
        ext_php_rs::describe::DocComments,
        ext_php_rs::flags::ConstantFlags,
    )] {
        &[
            (
                "MY_CONST",
                &42,
                &[" Doc comments for MY_CONST."],
                ::ext_php_rs::flags::ConstantFlags::Public,
            ),
        ]
    }
}
impl<'a> ::ext_php_rs::convert::FromZendObject<'a> for &'a PhpInterfaceMyInterface {
    #[inline]
    fn from_zend_object(
        obj: &'a ::ext_php_rs::types::ZendObject,
    ) -> ::ext_php_rs::error::Result<Self> {
        let obj = ::ext_php_rs::types::ZendClassObject::<
            PhpInterfaceMyInterface,
        >::from_zend_obj(obj)
            .ok_or(::ext_php_rs::error::Error::InvalidScope)?;
        Ok(&**obj)
    }
}
impl<'a> ::ext_php_rs::convert::FromZendObjectMut<'a>
for &'a mut PhpInterfaceMyInterface {
    #[inline]
    fn from_zend_object_mut(
        obj: &'a mut ::ext_php_rs::types::ZendObject,
    ) -> ::ext_php_rs::error::Result<Self> {
        let obj = ::ext_php_rs::types::ZendClassObject::<
            PhpInterfaceMyInterface,
        >::from_zend_obj_mut(obj)
            .ok_or(::ext_php_rs::error::Error::InvalidScope)?;
        Ok(&mut **obj)
    }
}
impl<'a> ::ext_php_rs::convert::FromZval<'a> for &'a PhpInterfaceMyInterface {
    const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::object(
        <PhpInterfaceMyInterface as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
    );
    #[inline]
    fn from_zval(zval: &'a ::ext_php_rs::types::Zval) -> ::std::option::Option<Self> {
        <Self as ::ext_php_rs::convert::FromZendObject>::from_zend_object(zval.object()?)
            .ok()
    }
}
impl<'a> ::ext_php_rs::convert::FromZvalMut<'a> for &'a mut PhpInterfaceMyInterface {
    const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::object(
        <PhpInterfaceMyInterface as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
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
impl ::ext_php_rs::convert::IntoZendObject for PhpInterfaceMyInterface {
    #[inline]
    fn into_zend_object(
        self,
    ) -> ::ext_php_rs::error::Result<
        ::ext_php_rs::boxed::ZBox<::ext_php_rs::types::ZendObject>,
    > {
        Ok(::ext_php_rs::types::ZendClassObject::new(self).into())
    }
}
impl ::ext_php_rs::convert::IntoZval for PhpInterfaceMyInterface {
    const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::object(
        <PhpInterfaceMyInterface as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
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
trait MyInterface2 {
    const MY_CONST: i32 = 42;
    const ANOTHER_CONST: &'static str = "Hello";
    fn my_method(&self, arg: i32) -> String;
    fn anotherMethod(&self) -> i32;
}
pub struct PhpInterfaceMyInterface2;
impl ::ext_php_rs::class::RegisteredClass for PhpInterfaceMyInterface2 {
    const CLASS_NAME: &'static str = "MyInterface2";
    const BUILDER_MODIFIER: Option<
        fn(::ext_php_rs::builders::ClassBuilder) -> ::ext_php_rs::builders::ClassBuilder,
    > = None;
    const EXTENDS: Option<::ext_php_rs::class::ClassEntryInfo> = None;
    const FLAGS: ::ext_php_rs::flags::ClassFlags = ::ext_php_rs::flags::ClassFlags::Interface;
    const IMPLEMENTS: &'static [::ext_php_rs::class::ClassEntryInfo] = &[];
    const DOC_COMMENTS: &'static [&'static str] = &[];
    fn get_metadata() -> &'static ::ext_php_rs::class::ClassMetadata<Self> {
        static METADATA: ::ext_php_rs::class::ClassMetadata<PhpInterfaceMyInterface2> = ::ext_php_rs::class::ClassMetadata::new(
            &[],
        );
        &METADATA
    }
    fn method_builders() -> Vec<
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
                                &[
                                    <i32 as ::ext_php_rs::convert::FromZvalMut>::NULLABLE
                                        || false,
                                ],
                            );
                            ::ext_php_rs::builders::FunctionBuilder::new_abstract(
                                    "MY_METHOD",
                                )
                                .arg(::ext_php_rs::args::Arg::of::<i32>("arg"))
                                .required_args(__REQUIRED)
                                .returns(
                                    <String as ::ext_php_rs::convert::IntoZval>::TYPE,
                                    false,
                                    <String as ::ext_php_rs::convert::IntoZval>::NULLABLE,
                                )
                        },
                        ::ext_php_rs::flags::MethodFlags::Public
                            | ::ext_php_rs::flags::MethodFlags::Abstract,
                    ),
                    (
                        {
                            const __REQUIRED: usize = ::ext_php_rs::args::required_count(
                                &[],
                            );
                            ::ext_php_rs::builders::FunctionBuilder::new_abstract(
                                    "AnotherMethod",
                                )
                                .required_args(__REQUIRED)
                                .returns(
                                    <i32 as ::ext_php_rs::convert::IntoZval>::TYPE,
                                    false,
                                    <i32 as ::ext_php_rs::convert::IntoZval>::NULLABLE,
                                )
                        },
                        ::ext_php_rs::flags::MethodFlags::Public
                            | ::ext_php_rs::flags::MethodFlags::Abstract,
                    ),
                ],
            ),
        )
    }
    fn constructor() -> Option<::ext_php_rs::class::ConstructorMeta<Self>> {
        None
    }
    fn constants() -> &'static [(
        &'static str,
        &'static dyn ext_php_rs::convert::IntoZvalDyn,
        ext_php_rs::describe::DocComments,
        ext_php_rs::flags::ConstantFlags,
    )] {
        &[
            ("my_const", &42, &[], ::ext_php_rs::flags::ConstantFlags::Public),
            ("AnotherConst", &"Hello", &[], ::ext_php_rs::flags::ConstantFlags::Public),
        ]
    }
}
impl<'a> ::ext_php_rs::convert::FromZendObject<'a> for &'a PhpInterfaceMyInterface2 {
    #[inline]
    fn from_zend_object(
        obj: &'a ::ext_php_rs::types::ZendObject,
    ) -> ::ext_php_rs::error::Result<Self> {
        let obj = ::ext_php_rs::types::ZendClassObject::<
            PhpInterfaceMyInterface2,
        >::from_zend_obj(obj)
            .ok_or(::ext_php_rs::error::Error::InvalidScope)?;
        Ok(&**obj)
    }
}
impl<'a> ::ext_php_rs::convert::FromZendObjectMut<'a>
for &'a mut PhpInterfaceMyInterface2 {
    #[inline]
    fn from_zend_object_mut(
        obj: &'a mut ::ext_php_rs::types::ZendObject,
    ) -> ::ext_php_rs::error::Result<Self> {
        let obj = ::ext_php_rs::types::ZendClassObject::<
            PhpInterfaceMyInterface2,
        >::from_zend_obj_mut(obj)
            .ok_or(::ext_php_rs::error::Error::InvalidScope)?;
        Ok(&mut **obj)
    }
}
impl<'a> ::ext_php_rs::convert::FromZval<'a> for &'a PhpInterfaceMyInterface2 {
    const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::object(
        <PhpInterfaceMyInterface2 as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
    );
    #[inline]
    fn from_zval(zval: &'a ::ext_php_rs::types::Zval) -> ::std::option::Option<Self> {
        <Self as ::ext_php_rs::convert::FromZendObject>::from_zend_object(zval.object()?)
            .ok()
    }
}
impl<'a> ::ext_php_rs::convert::FromZvalMut<'a> for &'a mut PhpInterfaceMyInterface2 {
    const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::object(
        <PhpInterfaceMyInterface2 as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
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
impl ::ext_php_rs::convert::IntoZendObject for PhpInterfaceMyInterface2 {
    #[inline]
    fn into_zend_object(
        self,
    ) -> ::ext_php_rs::error::Result<
        ::ext_php_rs::boxed::ZBox<::ext_php_rs::types::ZendObject>,
    > {
        Ok(::ext_php_rs::types::ZendClassObject::new(self).into())
    }
}
impl ::ext_php_rs::convert::IntoZval for PhpInterfaceMyInterface2 {
    const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::object(
        <PhpInterfaceMyInterface2 as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
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
struct MyImpl {}
impl ::ext_php_rs::class::RegisteredClass for MyImpl {
    const CLASS_NAME: &'static str = "MyImpl";
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
            MyImpl,
        >; 0usize] = [];
        static METADATA: ::ext_php_rs::class::ClassMetadata<MyImpl> = ::ext_php_rs::class::ClassMetadata::new(
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
impl MyInterface for MyImpl {
    fn my_method(&self, arg: i32) -> String {
        String::new()
    }
}
impl ::ext_php_rs::internal::class::InterfaceMethodsProvider<MyImpl>
for ::ext_php_rs::internal::class::PhpClassImplCollector<MyImpl> {
    fn get_interface_methods(
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
                                &[<i32 as ::ext_php_rs::convert::FromZvalMut>::NULLABLE],
                            );
                            ::ext_php_rs::builders::FunctionBuilder::new(
                                    "myMethod",
                                    {
                                        (/*ERROR*/);
                                        handler
                                    },
                                )
                                .arg(::ext_php_rs::args::Arg::of::<i32>("arg"))
                                .required_args(__REQUIRED)
                                .returns(
                                    <String as ::ext_php_rs::convert::IntoZval>::TYPE,
                                    false,
                                    <String as ::ext_php_rs::convert::IntoZval>::NULLABLE,
                                )
                        },
                        ::ext_php_rs::flags::MethodFlags::Public,
                    ),
                ],
            ),
        )
    }
}
