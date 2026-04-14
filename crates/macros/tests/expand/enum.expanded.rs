#[macro_use]
extern crate ext_php_rs_derive;
#[allow(dead_code)]
/// Doc comments for MyEnum.
/// This is a basic enum example.
enum MyEnum {
    /// Variant1 of MyEnum.
    /// This variant represents the first case.
    Variant1,
    Variant2,
    /// Variant3 of MyEnum.
    Variant3,
}
impl ::ext_php_rs::class::RegisteredClass for MyEnum {
    const CLASS_NAME: &'static str = "MyEnum";
    const BUILDER_MODIFIER: ::std::option::Option<
        fn(::ext_php_rs::builders::ClassBuilder) -> ::ext_php_rs::builders::ClassBuilder,
    > = None;
    const EXTENDS: ::std::option::Option<::ext_php_rs::class::ClassEntryInfo> = None;
    const IMPLEMENTS: &'static [::ext_php_rs::class::ClassEntryInfo] = &[];
    const FLAGS: ::ext_php_rs::flags::ClassFlags = ::ext_php_rs::flags::ClassFlags::Enum;
    const DOC_COMMENTS: &'static [&'static str] = &[
        " Doc comments for MyEnum.",
        " This is a basic enum example.",
    ];
    fn get_metadata() -> &'static ::ext_php_rs::class::ClassMetadata<Self> {
        static METADATA: ::ext_php_rs::class::ClassMetadata<MyEnum> = ::ext_php_rs::class::ClassMetadata::new(&[]);
        &METADATA
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
        None
    }
    #[inline]
    fn constants() -> &'static [(
        &'static str,
        &'static dyn ::ext_php_rs::convert::IntoZvalDyn,
        &'static [&'static str],
    )] {
        use ::ext_php_rs::internal::class::PhpClassImpl;
        ::ext_php_rs::internal::class::PhpClassImplCollector::<Self>::default()
            .get_constants()
    }
}
impl ::ext_php_rs::enum_::RegisteredEnum for MyEnum {
    const CASES: &'static [::ext_php_rs::enum_::EnumCase] = &[
        ::ext_php_rs::enum_::EnumCase {
            name: "Variant1",
            discriminant: None,
            docs: &[" Variant1 of MyEnum.", " This variant represents the first case."],
        },
        ::ext_php_rs::enum_::EnumCase {
            name: "Variant_2",
            discriminant: None,
            docs: &[],
        },
        ::ext_php_rs::enum_::EnumCase {
            name: "VARIANT_3",
            discriminant: None,
            docs: &[" Variant3 of MyEnum."],
        },
    ];
    fn from_name(name: &str) -> ::ext_php_rs::error::Result<Self> {
        match name {
            "Variant1" => Ok(Self::Variant1),
            "Variant_2" => Ok(Self::Variant2),
            "VARIANT_3" => Ok(Self::Variant3),
            _ => Err(::ext_php_rs::error::Error::InvalidProperty),
        }
    }
    fn to_name(&self) -> &'static str {
        match self {
            Self::Variant1 => "Variant1",
            Self::Variant2 => "Variant_2",
            Self::Variant3 => "VARIANT_3",
        }
    }
}
#[allow(dead_code)]
enum MyEnumWithIntValues {
    Variant1,
    Variant2,
}
impl ::ext_php_rs::class::RegisteredClass for MyEnumWithIntValues {
    const CLASS_NAME: &'static str = "MyIntValuesEnum";
    const BUILDER_MODIFIER: ::std::option::Option<
        fn(::ext_php_rs::builders::ClassBuilder) -> ::ext_php_rs::builders::ClassBuilder,
    > = None;
    const EXTENDS: ::std::option::Option<::ext_php_rs::class::ClassEntryInfo> = None;
    const IMPLEMENTS: &'static [::ext_php_rs::class::ClassEntryInfo] = &[];
    const FLAGS: ::ext_php_rs::flags::ClassFlags = ::ext_php_rs::flags::ClassFlags::Enum;
    const DOC_COMMENTS: &'static [&'static str] = &[];
    fn get_metadata() -> &'static ::ext_php_rs::class::ClassMetadata<Self> {
        static METADATA: ::ext_php_rs::class::ClassMetadata<MyEnumWithIntValues> = ::ext_php_rs::class::ClassMetadata::new(&[]);
        &METADATA
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
        None
    }
    #[inline]
    fn constants() -> &'static [(
        &'static str,
        &'static dyn ::ext_php_rs::convert::IntoZvalDyn,
        &'static [&'static str],
    )] {
        use ::ext_php_rs::internal::class::PhpClassImpl;
        ::ext_php_rs::internal::class::PhpClassImplCollector::<Self>::default()
            .get_constants()
    }
}
impl ::ext_php_rs::enum_::RegisteredEnum for MyEnumWithIntValues {
    const CASES: &'static [::ext_php_rs::enum_::EnumCase] = &[
        ::ext_php_rs::enum_::EnumCase {
            name: "Variant1",
            discriminant: Some(::ext_php_rs::enum_::Discriminant::Int(1i64)),
            docs: &[],
        },
        ::ext_php_rs::enum_::EnumCase {
            name: "Variant2",
            discriminant: Some(::ext_php_rs::enum_::Discriminant::Int(42i64)),
            docs: &[],
        },
    ];
    fn from_name(name: &str) -> ::ext_php_rs::error::Result<Self> {
        match name {
            "Variant1" => Ok(Self::Variant1),
            "Variant2" => Ok(Self::Variant2),
            _ => Err(::ext_php_rs::error::Error::InvalidProperty),
        }
    }
    fn to_name(&self) -> &'static str {
        match self {
            Self::Variant1 => "Variant1",
            Self::Variant2 => "Variant2",
        }
    }
}
impl TryFrom<i64> for MyEnumWithIntValues {
    type Error = ::ext_php_rs::error::Error;
    fn try_from(value: i64) -> ::ext_php_rs::error::Result<Self> {
        match value {
            1i64 => Ok(Self::Variant1),
            42i64 => Ok(Self::Variant2),
            _ => Err(::ext_php_rs::error::Error::InvalidProperty),
        }
    }
}
impl Into<i64> for MyEnumWithIntValues {
    fn into(self) -> i64 {
        match self {
            Self::Variant1 => 1i64,
            Self::Variant2 => 42i64,
        }
    }
}
#[allow(dead_code)]
enum MyEnumWithStringValues {
    Variant1,
    Variant2,
}
impl ::ext_php_rs::class::RegisteredClass for MyEnumWithStringValues {
    const CLASS_NAME: &'static str = "MY_ENUM_WITH_STRING_VALUES";
    const BUILDER_MODIFIER: ::std::option::Option<
        fn(::ext_php_rs::builders::ClassBuilder) -> ::ext_php_rs::builders::ClassBuilder,
    > = None;
    const EXTENDS: ::std::option::Option<::ext_php_rs::class::ClassEntryInfo> = None;
    const IMPLEMENTS: &'static [::ext_php_rs::class::ClassEntryInfo] = &[];
    const FLAGS: ::ext_php_rs::flags::ClassFlags = ::ext_php_rs::flags::ClassFlags::Enum;
    const DOC_COMMENTS: &'static [&'static str] = &[];
    fn get_metadata() -> &'static ::ext_php_rs::class::ClassMetadata<Self> {
        static METADATA: ::ext_php_rs::class::ClassMetadata<MyEnumWithStringValues> = ::ext_php_rs::class::ClassMetadata::new(&[]);
        &METADATA
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
        None
    }
    #[inline]
    fn constants() -> &'static [(
        &'static str,
        &'static dyn ::ext_php_rs::convert::IntoZvalDyn,
        &'static [&'static str],
    )] {
        use ::ext_php_rs::internal::class::PhpClassImpl;
        ::ext_php_rs::internal::class::PhpClassImplCollector::<Self>::default()
            .get_constants()
    }
}
impl ::ext_php_rs::enum_::RegisteredEnum for MyEnumWithStringValues {
    const CASES: &'static [::ext_php_rs::enum_::EnumCase] = &[
        ::ext_php_rs::enum_::EnumCase {
            name: "Variant1",
            discriminant: Some(::ext_php_rs::enum_::Discriminant::String("foo")),
            docs: &[],
        },
        ::ext_php_rs::enum_::EnumCase {
            name: "Variant2",
            discriminant: Some(::ext_php_rs::enum_::Discriminant::String("bar")),
            docs: &[],
        },
    ];
    fn from_name(name: &str) -> ::ext_php_rs::error::Result<Self> {
        match name {
            "Variant1" => Ok(Self::Variant1),
            "Variant2" => Ok(Self::Variant2),
            _ => Err(::ext_php_rs::error::Error::InvalidProperty),
        }
    }
    fn to_name(&self) -> &'static str {
        match self {
            Self::Variant1 => "Variant1",
            Self::Variant2 => "Variant2",
        }
    }
}
impl TryFrom<&str> for MyEnumWithStringValues {
    type Error = ::ext_php_rs::error::Error;
    fn try_from(value: &str) -> ::ext_php_rs::error::Result<Self> {
        match value {
            "foo" => Ok(Self::Variant1),
            "bar" => Ok(Self::Variant2),
            _ => Err(::ext_php_rs::error::Error::InvalidProperty),
        }
    }
}
impl Into<&'static str> for MyEnumWithStringValues {
    fn into(self) -> &'static str {
        match self {
            Self::Variant1 => "foo",
            Self::Variant2 => "bar",
        }
    }
}
