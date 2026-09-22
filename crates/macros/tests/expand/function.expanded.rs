#[macro_use]
extern crate ext_php_rs_derive;
/// Doc comments for greet.
pub fn greet(name: String, age: Option<i64>, times: i64) -> String {
    String::new()
}
#[doc(hidden)]
#[allow(non_camel_case_types)]
struct _internal_greet;
impl ::ext_php_rs::internal::function::PhpFunction for _internal_greet {
    const FUNCTION_ENTRY: fn() -> ::ext_php_rs::builders::FunctionBuilder<'static> = {
        fn entry() -> ::ext_php_rs::builders::FunctionBuilder<'static> {
            {
                const __REQUIRED: usize = ::ext_php_rs::args::required_count(
                    &[
                        <String as ::ext_php_rs::convert::FromZvalMut>::NULLABLE
                            || false,
                        <Option<i64> as ::ext_php_rs::convert::FromZvalMut>::NULLABLE
                            || false,
                        <i64 as ::ext_php_rs::convert::FromZvalMut>::NULLABLE || true,
                    ],
                );
                ::ext_php_rs::builders::FunctionBuilder::new(
                        "greet",
                        {
                            (/*ERROR*/);
                            handler
                        },
                    )
                    .arg(::ext_php_rs::args::Arg::of::<String>("name"))
                    .arg(::ext_php_rs::args::Arg::of::<Option<i64>>("age"))
                    .arg(
                        ::ext_php_rs::args::Arg::of::<i64>("times")
                            .default({
                                let __default: i64 = (1).into();
                                ::ext_php_rs::convert::StubLiteral::stub_literal(&__default)
                            }),
                    )
                    .required_args(__REQUIRED)
                    .returns(
                        <String as ::ext_php_rs::convert::IntoZval>::TYPE,
                        false,
                        <String as ::ext_php_rs::convert::IntoZval>::NULLABLE,
                    )
                    .docs(&[" Doc comments for greet."])
            }
        }
        entry
    };
}
pub fn sum(first: i64, rest: &[i64]) -> i64 {
    first
}
#[doc(hidden)]
#[allow(non_camel_case_types)]
struct _internal_sum;
impl ::ext_php_rs::internal::function::PhpFunction for _internal_sum {
    const FUNCTION_ENTRY: fn() -> ::ext_php_rs::builders::FunctionBuilder<'static> = {
        fn entry() -> ::ext_php_rs::builders::FunctionBuilder<'static> {
            {
                const __REQUIRED: usize = ::ext_php_rs::args::required_count(
                    &[
                        <i64 as ::ext_php_rs::convert::FromZvalMut>::NULLABLE || false,
                        false,
                    ],
                );
                ::ext_php_rs::builders::FunctionBuilder::new(
                        "sum",
                        {
                            (/*ERROR*/);
                            handler
                        },
                    )
                    .arg(::ext_php_rs::args::Arg::of::<i64>("first"))
                    .arg(::ext_php_rs::args::Arg::of::<i64>("rest").is_variadic())
                    .required_args(__REQUIRED)
                    .returns(
                        <i64 as ::ext_php_rs::convert::IntoZval>::TYPE,
                        false,
                        <i64 as ::ext_php_rs::convert::IntoZval>::NULLABLE,
                    )
            }
        }
        entry
    };
}
pub fn bump(target: ext_php_rs::types::PhpRef<'_>) {}
#[doc(hidden)]
#[allow(non_camel_case_types)]
struct _internal_bump;
impl ::ext_php_rs::internal::function::PhpFunction for _internal_bump {
    const FUNCTION_ENTRY: fn() -> ::ext_php_rs::builders::FunctionBuilder<'static> = {
        fn entry() -> ::ext_php_rs::builders::FunctionBuilder<'static> {
            {
                const __REQUIRED: usize = ::ext_php_rs::args::required_count(
                    &[
                        <ext_php_rs::types::PhpRef as ::ext_php_rs::convert::FromZvalMut>::NULLABLE
                            || false,
                    ],
                );
                ::ext_php_rs::builders::FunctionBuilder::new(
                        "bump",
                        {
                            (/*ERROR*/);
                            handler
                        },
                    )
                    .arg(
                        ::ext_php_rs::args::Arg::of::<
                            ext_php_rs::types::PhpRef,
                        >("target"),
                    )
                    .required_args(__REQUIRED)
                    .returns(::ext_php_rs::flags::DataType::Void, false, false)
            }
        }
        entry
    };
}
