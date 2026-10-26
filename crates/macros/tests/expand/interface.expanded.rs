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
impl PhpInterfaceMyInterface {
    #[doc(hidden)]
    #[allow(non_upper_case_globals)]
    pub const __php_method_my_method: &'static str = "myMethod";
}
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
                                    ::ext_php_rs::internal::argument::nullable::<i32, _>()
                                        || false,
                                ],
                            );
                            ::ext_php_rs::builders::FunctionBuilder::new_abstract(
                                    "myMethod",
                                )
                                .arg(::ext_php_rs::internal::argument::arg::<i32, _>("arg"))
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
    fn static_method() -> i32;
}
pub struct PhpInterfaceMyInterface2;
impl PhpInterfaceMyInterface2 {
    #[doc(hidden)]
    #[allow(non_upper_case_globals)]
    pub const __php_method_my_method: &'static str = "MY_METHOD";
    #[doc(hidden)]
    #[allow(non_upper_case_globals)]
    pub const __php_method_anotherMethod: &'static str = "AnotherMethod";
    #[doc(hidden)]
    #[allow(non_upper_case_globals)]
    pub const __php_method_static_method: &'static str = "STATIC_METHOD";
}
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
                                    ::ext_php_rs::internal::argument::nullable::<i32, _>()
                                        || false,
                                ],
                            );
                            ::ext_php_rs::builders::FunctionBuilder::new_abstract(
                                    "MY_METHOD",
                                )
                                .arg(::ext_php_rs::internal::argument::arg::<i32, _>("arg"))
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
                    (
                        {
                            const __REQUIRED: usize = ::ext_php_rs::args::required_count(
                                &[],
                            );
                            ::ext_php_rs::builders::FunctionBuilder::new_abstract(
                                    "STATIC_METHOD",
                                )
                                .required_args(__REQUIRED)
                                .returns(
                                    <i32 as ::ext_php_rs::convert::IntoZval>::TYPE,
                                    false,
                                    <i32 as ::ext_php_rs::convert::IntoZval>::NULLABLE,
                                )
                        },
                        ::ext_php_rs::flags::MethodFlags::Public
                            | ::ext_php_rs::flags::MethodFlags::Abstract
                            | ::ext_php_rs::flags::MethodFlags::Static,
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
    #[inline]
    #[must_use]
    fn cloneable() -> bool {
        use ::ext_php_rs::internal::class::ProbeClone as _;
        ::ext_php_rs::internal::class::CloneProbe::<Self>::default().cloneable()
    }
}
impl ::ext_php_rs::convert::IntoZendObject for MyImpl {
    #[inline]
    fn into_zend_object(
        self,
    ) -> ::ext_php_rs::error::Result<
        ::ext_php_rs::boxed::ZBox<::ext_php_rs::types::ZendObject>,
    > {
        Ok(::ext_php_rs::types::ZendClassObject::new(self).into())
    }
}
impl ::ext_php_rs::convert::IntoZval for MyImpl {
    const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::object(
        <MyImpl as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
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
impl MyInterface for MyImpl {
    fn my_method(&self, arg: i32) -> String {
        String::new()
    }
}
#[allow(non_upper_case_globals)]
const _: () = {
    static __INVENTORY: ::inventory::Node = ::inventory::Node {
        value: &{
            ::ext_php_rs::internal::class::InterfaceRegistration {
                class_type_id: ::std::any::TypeId::of::<MyImpl>(),
                interface_getter: || (
                    || {
                        <PhpInterfaceMyInterface as ::ext_php_rs::class::RegisteredClass>::get_metadata()
                            .ce()
                    },
                    <PhpInterfaceMyInterface as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
                ),
            }
        },
        next: ::inventory::__private::UnsafeCell::new(
            ::inventory::__private::Option::None,
        ),
    };
    #[link_section = ".text.startup"]
    unsafe extern "C" fn __ctor() {
        unsafe { ::inventory::ErasedNode::submit(__INVENTORY.value, &__INVENTORY) }
    }
    #[used]
    #[link_section = ".init_array"]
    static __CTOR: unsafe extern "C" fn() = __ctor;
};
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
                                &[::ext_php_rs::internal::argument::nullable::<i32, _>()],
                            );
                            ::ext_php_rs::builders::FunctionBuilder::new(
                                    <PhpInterfaceMyInterface>::__php_method_my_method,
                                    {
                                        extern "C" fn handler(
                                            ex: &mut ::ext_php_rs::zend::ExecuteData,
                                            retval: &mut ::ext_php_rs::types::Zval,
                                        ) {
                                            use ::ext_php_rs::convert::IntoZval;
                                            ::ext_php_rs::zend::run_handler(
                                                ::std::panic::AssertUnwindSafe(|| {
                                                    let (parse, this) = ex.parser_method::<MyImpl>();
                                                    let this = match this {
                                                        Some(this) => this,
                                                        None => {
                                                            ::ext_php_rs::exception::PhpException::from_message(
                                                                    "Failed to get $this".into(),
                                                                )
                                                                .throw();
                                                            return;
                                                        }
                                                    };
                                                    let __payload = match this.try_borrow() {
                                                        Ok(payload) => payload,
                                                        Err(err) => {
                                                            ::ext_php_rs::exception::PhpException::from(err).throw();
                                                            return;
                                                        }
                                                    };
                                                    let mut arg = ::ext_php_rs::internal::argument::arg::<
                                                        i32,
                                                        _,
                                                    >("arg");
                                                    #[allow(non_snake_case)]
                                                    let mut __holder_arg = ::ext_php_rs::internal::argument::holder::<
                                                        i32,
                                                        _,
                                                    >();
                                                    let parse_result = parse
                                                        .arg(&mut arg)
                                                        .required_args(__REQUIRED)
                                                        .parse();
                                                    if parse_result.is_err() {
                                                        return;
                                                    }
                                                    let arg: i32 = match arg.zval() {
                                                        Some(zval) => {
                                                            match ::ext_php_rs::internal::argument::extract::<
                                                                i32,
                                                                _,
                                                            >(zval.dereference_mut(), &mut __holder_arg) {
                                                                Ok(value) => value,
                                                                Err(error) => {
                                                                    ::ext_php_rs::internal::argument::argument_error(
                                                                            "arg",
                                                                            &error,
                                                                        )
                                                                        .throw();
                                                                    return;
                                                                }
                                                            }
                                                        }
                                                        None => {
                                                            match ::ext_php_rs::internal::argument::missing::<
                                                                i32,
                                                                _,
                                                            >() {
                                                                Some(value) => value,
                                                                None => {
                                                                    ::ext_php_rs::exception::PhpException::new(
                                                                            "Invalid value given for argument `arg`.".into(),
                                                                            0,
                                                                            ::ext_php_rs::zend::ce::type_error(),
                                                                        )
                                                                        .throw();
                                                                    return;
                                                                }
                                                            }
                                                        }
                                                    };
                                                    let result = __payload.my_method(arg);
                                                    if let Err(e) = result.set_zval(retval, false) {
                                                        let e: ::ext_php_rs::exception::PhpException = e.into();
                                                        e.throw();
                                                    }
                                                }),
                                            );
                                        }
                                        handler
                                    },
                                )
                                .arg(::ext_php_rs::internal::argument::arg::<i32, _>("arg"))
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
