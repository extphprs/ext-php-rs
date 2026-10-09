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
                            #[allow(clippy::used_underscore_binding)]
                            extern "C" fn handler(
                                ex: &mut ::ext_php_rs::zend::ExecuteData,
                                retval: &mut ::ext_php_rs::types::Zval,
                            ) {
                                ::ext_php_rs::zend::run_handler(
                                    ::std::panic::AssertUnwindSafe(|| {
                                        use ::ext_php_rs::convert::{FromZvalMut, IntoZval};
                                        let __num_args = unsafe { ex.This.u2.num_args } as usize;
                                        if !(__REQUIRED..=3usize).contains(&__num_args) {
                                            unsafe {
                                                ::ext_php_rs::ffi::zend_wrong_parameters_count_error(
                                                    __REQUIRED.try_into().unwrap_or(u32::MAX),
                                                    3u32,
                                                );
                                            };
                                            return;
                                        }
                                        let name: String = {
                                            let __value = if 0usize < __REQUIRED || __num_args > 0usize
                                            {
                                                let __zval_0 = unsafe { ex.zend_call_arg(0usize) };
                                                let Some(__zval_0) = __zval_0 else {
                                                    return;
                                                };
                                                <String as ::ext_php_rs::convert::FromZvalMut>::from_zval_mut(
                                                    __zval_0.dereference_mut(),
                                                )
                                            } else {
                                                <String as ::ext_php_rs::convert::FromZvalMut>::from_missing()
                                            };
                                            match __value {
                                                Some(value) => value,
                                                None => {
                                                    ::ext_php_rs::exception::PhpException::from_message(
                                                            "Invalid value given for argument `name`.".into(),
                                                        )
                                                        .throw();
                                                    return;
                                                }
                                            }
                                        };
                                        let age: Option<i64> = {
                                            let __value = if 1usize < __REQUIRED || __num_args > 1usize
                                            {
                                                let __zval_1 = unsafe { ex.zend_call_arg(1usize) };
                                                let Some(__zval_1) = __zval_1 else {
                                                    return;
                                                };
                                                <Option<
                                                    i64,
                                                > as ::ext_php_rs::convert::FromZvalMut>::from_zval_mut(
                                                    __zval_1.dereference_mut(),
                                                )
                                            } else {
                                                <Option<
                                                    i64,
                                                > as ::ext_php_rs::convert::FromZvalMut>::from_missing()
                                            };
                                            match __value {
                                                Some(value) => value,
                                                None => {
                                                    ::ext_php_rs::exception::PhpException::from_message(
                                                            "Invalid value given for argument `age`.".into(),
                                                        )
                                                        .throw();
                                                    return;
                                                }
                                            }
                                        };
                                        let times: i64 = {
                                            let __value = if 2usize < __REQUIRED || __num_args > 2usize
                                            {
                                                let __zval_2 = unsafe { ex.zend_call_arg(2usize) };
                                                let Some(__zval_2) = __zval_2 else {
                                                    return;
                                                };
                                                if !<i64 as ::ext_php_rs::convert::FromZvalMut>::NULLABLE
                                                    && __zval_2.is_null()
                                                {
                                                    ::ext_php_rs::exception::PhpException::new(
                                                            "Argument `$times` must not be null".into(),
                                                            0,
                                                            ::ext_php_rs::zend::ce::type_error(),
                                                        )
                                                        .throw();
                                                    return;
                                                }
                                                <i64 as ::ext_php_rs::convert::FromZvalMut>::from_zval_mut(
                                                    __zval_2.dereference_mut(),
                                                )
                                            } else {
                                                ::std::option::Option::Some((1).into())
                                            };
                                            match __value {
                                                Some(value) => value,
                                                None => {
                                                    ::ext_php_rs::exception::PhpException::from_message(
                                                            "Invalid value given for argument `times`.".into(),
                                                        )
                                                        .throw();
                                                    return;
                                                }
                                            }
                                        };
                                        let __result = { greet(name, age, times) };
                                        if let Err(e) = __result.set_zval(retval, false) {
                                            let e: ::ext_php_rs::exception::PhpException = e.into();
                                            e.throw();
                                        }
                                    }),
                                );
                            }
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
                            #[allow(clippy::used_underscore_binding)]
                            extern "C" fn handler(
                                ex: &mut ::ext_php_rs::zend::ExecuteData,
                                retval: &mut ::ext_php_rs::types::Zval,
                            ) {
                                ::ext_php_rs::zend::run_handler(
                                    ::std::panic::AssertUnwindSafe(|| {
                                        use ::ext_php_rs::convert::IntoZval;
                                        let mut first = ::ext_php_rs::args::Arg::of::<i64>("first");
                                        let mut rest = ::ext_php_rs::args::Arg::of::<i64>("rest")
                                            .is_variadic();
                                        let result = {
                                            let parse = ex
                                                .parser()
                                                .arg(&mut first)
                                                .arg(&mut rest)
                                                .required_args(__REQUIRED)
                                                .parse();
                                            if parse.is_err() {
                                                return;
                                            }
                                            let __variadic_rest = rest.variadic_vals::<i64>();
                                            sum(
                                                {
                                                    match match first.zval() {
                                                        Some(zval) => {
                                                            <i64 as ::ext_php_rs::convert::FromZvalMut>::from_zval_mut(
                                                                zval.dereference_mut(),
                                                            )
                                                        }
                                                        None => {
                                                            <i64 as ::ext_php_rs::convert::FromZvalMut>::from_missing()
                                                        }
                                                    } {
                                                        Some(value) => value,
                                                        None => {
                                                            ::ext_php_rs::exception::PhpException::from_message(
                                                                    "Invalid value given for argument `first`.".into(),
                                                                )
                                                                .throw();
                                                            return;
                                                        }
                                                    }
                                                },
                                                { __variadic_rest.as_slice() },
                                            )
                                        };
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
                            #[allow(clippy::used_underscore_binding)]
                            extern "C" fn handler(
                                ex: &mut ::ext_php_rs::zend::ExecuteData,
                                retval: &mut ::ext_php_rs::types::Zval,
                            ) {
                                ::ext_php_rs::zend::run_handler(
                                    ::std::panic::AssertUnwindSafe(|| {
                                        use ::ext_php_rs::convert::{FromZvalMut, IntoZval};
                                        let __num_args = unsafe { ex.This.u2.num_args } as usize;
                                        if !(__REQUIRED..=1usize).contains(&__num_args) {
                                            unsafe {
                                                ::ext_php_rs::ffi::zend_wrong_parameters_count_error(
                                                    __REQUIRED.try_into().unwrap_or(u32::MAX),
                                                    1u32,
                                                );
                                            };
                                            return;
                                        }
                                        let target: ext_php_rs::types::PhpRef = {
                                            let __value = if 0usize < __REQUIRED || __num_args > 0usize
                                            {
                                                let __zval_0 = unsafe { ex.zend_call_arg(0usize) };
                                                let Some(__zval_0) = __zval_0 else {
                                                    return;
                                                };
                                                <ext_php_rs::types::PhpRef as ::ext_php_rs::convert::FromZvalMut>::from_zval_mut(
                                                    __zval_0.dereference_mut(),
                                                )
                                            } else {
                                                <ext_php_rs::types::PhpRef as ::ext_php_rs::convert::FromZvalMut>::from_missing()
                                            };
                                            match __value {
                                                Some(value) => value,
                                                None => {
                                                    ::ext_php_rs::exception::PhpException::from_message(
                                                            "Invalid value given for argument `target`.".into(),
                                                        )
                                                        .throw();
                                                    return;
                                                }
                                            }
                                        };
                                        let __result = { bump(target) };
                                        if let Err(e) = __result.set_zval(retval, false) {
                                            let e: ::ext_php_rs::exception::PhpException = e.into();
                                            e.throw();
                                        }
                                    }),
                                );
                            }
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
