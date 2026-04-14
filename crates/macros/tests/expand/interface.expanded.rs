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
        static METADATA: ::ext_php_rs::class::ClassMetadata<PhpInterfaceMyInterface> = ::ext_php_rs::class::ClassMetadata::new(&[]);
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
                        ::ext_php_rs::builders::FunctionBuilder::new_abstract("myMethod")
                            .arg(
                                ::ext_php_rs::args::Arg::new(
                                    "arg",
                                    <i32 as ::ext_php_rs::convert::FromZvalMut>::TYPE,
                                ),
                            )
                            .not_required()
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
                            ),
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
    )] {
        &[("MY_CONST", &42, &[" Doc comments for MY_CONST."])]
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
    const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::Object(
        Some(
            <PhpInterfaceMyInterface as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
        ),
    );
    #[inline]
    fn from_zval(zval: &'a ::ext_php_rs::types::Zval) -> ::std::option::Option<Self> {
        <Self as ::ext_php_rs::convert::FromZendObject>::from_zend_object(zval.object()?)
            .ok()
    }
}
impl<'a> ::ext_php_rs::convert::FromZvalMut<'a> for &'a mut PhpInterfaceMyInterface {
    const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::Object(
        Some(
            <PhpInterfaceMyInterface as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
        ),
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
    const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::Object(
        Some(
            <PhpInterfaceMyInterface as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
        ),
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
        static METADATA: ::ext_php_rs::class::ClassMetadata<PhpInterfaceMyInterface2> = ::ext_php_rs::class::ClassMetadata::new(&[]);
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
                        ::ext_php_rs::builders::FunctionBuilder::new_abstract(
                                "MY_METHOD",
                            )
                            .arg(
                                ::ext_php_rs::args::Arg::new(
                                    "arg",
                                    <i32 as ::ext_php_rs::convert::FromZvalMut>::TYPE,
                                ),
                            )
                            .not_required()
                            .returns(
                                <String as ::ext_php_rs::convert::IntoZval>::TYPE,
                                false,
                                <String as ::ext_php_rs::convert::IntoZval>::NULLABLE,
                            ),
                        ::ext_php_rs::flags::MethodFlags::Public
                            | ::ext_php_rs::flags::MethodFlags::Abstract,
                    ),
                    (
                        ::ext_php_rs::builders::FunctionBuilder::new_abstract(
                                "AnotherMethod",
                            )
                            .not_required()
                            .returns(
                                <i32 as ::ext_php_rs::convert::IntoZval>::TYPE,
                                false,
                                <i32 as ::ext_php_rs::convert::IntoZval>::NULLABLE,
                            ),
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
    )] {
        &[("my_const", &42, &[]), ("AnotherConst", &"Hello", &[])]
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
    const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::Object(
        Some(
            <PhpInterfaceMyInterface2 as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
        ),
    );
    #[inline]
    fn from_zval(zval: &'a ::ext_php_rs::types::Zval) -> ::std::option::Option<Self> {
        <Self as ::ext_php_rs::convert::FromZendObject>::from_zend_object(zval.object()?)
            .ok()
    }
}
impl<'a> ::ext_php_rs::convert::FromZvalMut<'a> for &'a mut PhpInterfaceMyInterface2 {
    const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::Object(
        Some(
            <PhpInterfaceMyInterface2 as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
        ),
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
    const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::Object(
        Some(
            <PhpInterfaceMyInterface2 as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
        ),
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
