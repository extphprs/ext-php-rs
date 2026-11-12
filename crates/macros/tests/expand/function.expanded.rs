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
                        ::ext_php_rs::internal::argument::nullable::<String, _>()
                            || false,
                        ::ext_php_rs::internal::argument::nullable::<Option<i64>, _>()
                            || false,
                        ::ext_php_rs::internal::argument::nullable::<i64, _>() || true,
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
                                        #[allow(non_snake_case)]
                                        let mut __holder_name = ::ext_php_rs::internal::argument::holder::<
                                            String,
                                            _,
                                        >();
                                        let name: String = if 0usize < __REQUIRED
                                            || __num_args > 0usize
                                        {
                                            let __zval_0 = unsafe { ex.zend_call_arg(0usize) };
                                            let Some(__zval_0) = __zval_0 else {
                                                return;
                                            };
                                            match ::ext_php_rs::internal::argument::extract::<
                                                String,
                                                _,
                                            >(__zval_0.dereference_mut(), &mut __holder_name) {
                                                Ok(value) => value,
                                                Err(error) => {
                                                    ::ext_php_rs::internal::argument::argument_error(
                                                            "name",
                                                            &error,
                                                        )
                                                        .throw();
                                                    return;
                                                }
                                            }
                                        } else {
                                            match ::ext_php_rs::internal::argument::missing::<
                                                String,
                                                _,
                                            >() {
                                                Some(value) => value,
                                                None => {
                                                    ::ext_php_rs::exception::PhpException::new(
                                                            "Invalid value given for argument `name`.".into(),
                                                            0,
                                                            ::ext_php_rs::zend::ce::type_error(),
                                                        )
                                                        .throw();
                                                    return;
                                                }
                                            }
                                        };
                                        #[allow(non_snake_case)]
                                        let mut __holder_age = ::ext_php_rs::internal::argument::holder::<
                                            Option<i64>,
                                            _,
                                        >();
                                        let age: Option<i64> = if 1usize < __REQUIRED
                                            || __num_args > 1usize
                                        {
                                            let __zval_1 = unsafe { ex.zend_call_arg(1usize) };
                                            let Some(__zval_1) = __zval_1 else {
                                                return;
                                            };
                                            match ::ext_php_rs::internal::argument::extract::<
                                                Option<i64>,
                                                _,
                                            >(__zval_1.dereference_mut(), &mut __holder_age) {
                                                Ok(value) => value,
                                                Err(error) => {
                                                    ::ext_php_rs::internal::argument::argument_error(
                                                            "age",
                                                            &error,
                                                        )
                                                        .throw();
                                                    return;
                                                }
                                            }
                                        } else {
                                            match ::ext_php_rs::internal::argument::missing::<
                                                Option<i64>,
                                                _,
                                            >() {
                                                Some(value) => value,
                                                None => {
                                                    ::ext_php_rs::exception::PhpException::new(
                                                            "Invalid value given for argument `age`.".into(),
                                                            0,
                                                            ::ext_php_rs::zend::ce::type_error(),
                                                        )
                                                        .throw();
                                                    return;
                                                }
                                            }
                                        };
                                        #[allow(non_snake_case)]
                                        let mut __holder_times = ::ext_php_rs::internal::argument::holder::<
                                            i64,
                                            _,
                                        >();
                                        let times: i64 = if 2usize < __REQUIRED
                                            || __num_args > 2usize
                                        {
                                            let __zval_2 = unsafe { ex.zend_call_arg(2usize) };
                                            let Some(__zval_2) = __zval_2 else {
                                                return;
                                            };
                                            if !::ext_php_rs::internal::argument::nullable::<i64, _>()
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
                                            match ::ext_php_rs::internal::argument::extract::<
                                                i64,
                                                _,
                                            >(__zval_2.dereference_mut(), &mut __holder_times) {
                                                Ok(value) => value,
                                                Err(error) => {
                                                    ::ext_php_rs::internal::argument::argument_error(
                                                            "times",
                                                            &error,
                                                        )
                                                        .throw();
                                                    return;
                                                }
                                            }
                                        } else {
                                            match ::std::option::Option::Some((1).into()) {
                                                Some(value) => value,
                                                None => {
                                                    ::ext_php_rs::exception::PhpException::new(
                                                            "Invalid value given for argument `times`.".into(),
                                                            0,
                                                            ::ext_php_rs::zend::ce::type_error(),
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
                    .arg(::ext_php_rs::internal::argument::arg::<String, _>("name"))
                    .arg(::ext_php_rs::internal::argument::arg::<Option<i64>, _>("age"))
                    .arg(
                        ::ext_php_rs::internal::argument::arg::<i64, _>("times")
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
                        ::ext_php_rs::internal::argument::nullable::<i64, _>() || false,
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
                                        let mut first = ::ext_php_rs::internal::argument::arg::<
                                            i64,
                                            _,
                                        >("first");
                                        let mut rest = ::ext_php_rs::internal::argument::arg::<
                                            i64,
                                            _,
                                        >("rest")
                                            .is_variadic();
                                        #[allow(non_snake_case)]
                                        let mut __holder_first = ::ext_php_rs::internal::argument::holder::<
                                            i64,
                                            _,
                                        >();
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
                                            let __variadic_rest = rest.try_variadic_vals::<i64>();
                                            sum(
                                                {
                                                    match first.zval() {
                                                        Some(zval) => {
                                                            match ::ext_php_rs::internal::argument::extract::<
                                                                i64,
                                                                _,
                                                            >(zval.dereference_mut(), &mut __holder_first) {
                                                                Ok(value) => value,
                                                                Err(error) => {
                                                                    ::ext_php_rs::internal::argument::argument_error(
                                                                            "first",
                                                                            &error,
                                                                        )
                                                                        .throw();
                                                                    return;
                                                                }
                                                            }
                                                        }
                                                        None => {
                                                            match ::ext_php_rs::internal::argument::missing::<
                                                                i64,
                                                                _,
                                                            >() {
                                                                Some(value) => value,
                                                                None => {
                                                                    ::ext_php_rs::exception::PhpException::new(
                                                                            "Invalid value given for argument `first`.".into(),
                                                                            0,
                                                                            ::ext_php_rs::zend::ce::type_error(),
                                                                        )
                                                                        .throw();
                                                                    return;
                                                                }
                                                            }
                                                        }
                                                    }
                                                },
                                                {
                                                    match __variadic_rest.as_deref() {
                                                        Ok(values) => values,
                                                        Err(error) => {
                                                            ::ext_php_rs::internal::argument::argument_error(
                                                                    "rest",
                                                                    error,
                                                                )
                                                                .throw();
                                                            return;
                                                        }
                                                    }
                                                },
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
                    .arg(::ext_php_rs::internal::argument::arg::<i64, _>("first"))
                    .arg(
                        ::ext_php_rs::internal::argument::arg::<i64, _>("rest")
                            .is_variadic(),
                    )
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
                        ::ext_php_rs::internal::argument::nullable::<
                            ext_php_rs::types::PhpRef,
                            _,
                        >() || false,
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
                                        #[allow(non_snake_case)]
                                        let mut __holder_target = ::ext_php_rs::internal::argument::holder::<
                                            ext_php_rs::types::PhpRef,
                                            _,
                                        >();
                                        let target: ext_php_rs::types::PhpRef = if 0usize
                                            < __REQUIRED || __num_args > 0usize
                                        {
                                            let __zval_0 = unsafe { ex.zend_call_arg(0usize) };
                                            let Some(__zval_0) = __zval_0 else {
                                                return;
                                            };
                                            match ::ext_php_rs::internal::argument::extract::<
                                                ext_php_rs::types::PhpRef,
                                                _,
                                            >(__zval_0.dereference_mut(), &mut __holder_target) {
                                                Ok(value) => value,
                                                Err(error) => {
                                                    ::ext_php_rs::internal::argument::argument_error(
                                                            "target",
                                                            &error,
                                                        )
                                                        .throw();
                                                    return;
                                                }
                                            }
                                        } else {
                                            match ::ext_php_rs::internal::argument::missing::<
                                                ext_php_rs::types::PhpRef,
                                                _,
                                            >() {
                                                Some(value) => value,
                                                None => {
                                                    ::ext_php_rs::exception::PhpException::new(
                                                            "Invalid value given for argument `target`.".into(),
                                                            0,
                                                            ::ext_php_rs::zend::ce::type_error(),
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
                        ::ext_php_rs::internal::argument::arg::<
                            ext_php_rs::types::PhpRef,
                            _,
                        >("target"),
                    )
                    .required_args(__REQUIRED)
                    .returns(::ext_php_rs::flags::DataType::Void, false, false)
            }
        }
        entry
    };
}
pub fn merge(target: &mut Counter, source: Option<&Counter>) {}
#[doc(hidden)]
#[allow(non_camel_case_types)]
struct _internal_merge;
impl ::ext_php_rs::internal::function::PhpFunction for _internal_merge {
    const FUNCTION_ENTRY: fn() -> ::ext_php_rs::builders::FunctionBuilder<'static> = {
        fn entry() -> ::ext_php_rs::builders::FunctionBuilder<'static> {
            {
                const __REQUIRED: usize = ::ext_php_rs::args::required_count(
                    &[
                        ::ext_php_rs::internal::argument::nullable::<&mut Counter, _>()
                            || false,
                        ::ext_php_rs::internal::argument::nullable::<
                            Option<&Counter>,
                            _,
                        >() || false,
                    ],
                );
                ::ext_php_rs::builders::FunctionBuilder::new(
                        "merge",
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
                                        if !(__REQUIRED..=2usize).contains(&__num_args) {
                                            unsafe {
                                                ::ext_php_rs::ffi::zend_wrong_parameters_count_error(
                                                    __REQUIRED.try_into().unwrap_or(u32::MAX),
                                                    2u32,
                                                );
                                            };
                                            return;
                                        }
                                        #[allow(non_snake_case)]
                                        let mut __holder_target = ::ext_php_rs::internal::argument::holder::<
                                            &mut Counter,
                                            _,
                                        >();
                                        let target: &mut Counter = if 0usize < __REQUIRED
                                            || __num_args > 0usize
                                        {
                                            let __zval_0 = unsafe { ex.zend_call_arg(0usize) };
                                            let Some(__zval_0) = __zval_0 else {
                                                return;
                                            };
                                            match ::ext_php_rs::internal::argument::extract::<
                                                &mut Counter,
                                                _,
                                            >(__zval_0.dereference_mut(), &mut __holder_target) {
                                                Ok(value) => value,
                                                Err(error) => {
                                                    ::ext_php_rs::internal::argument::argument_error(
                                                            "target",
                                                            &error,
                                                        )
                                                        .throw();
                                                    return;
                                                }
                                            }
                                        } else {
                                            match ::ext_php_rs::internal::argument::missing::<
                                                &mut Counter,
                                                _,
                                            >() {
                                                Some(value) => value,
                                                None => {
                                                    ::ext_php_rs::exception::PhpException::new(
                                                            "Invalid value given for argument `target`.".into(),
                                                            0,
                                                            ::ext_php_rs::zend::ce::type_error(),
                                                        )
                                                        .throw();
                                                    return;
                                                }
                                            }
                                        };
                                        #[allow(non_snake_case)]
                                        let mut __holder_source = ::ext_php_rs::internal::argument::holder::<
                                            Option<&Counter>,
                                            _,
                                        >();
                                        let source: Option<&Counter> = if 1usize < __REQUIRED
                                            || __num_args > 1usize
                                        {
                                            let __zval_1 = unsafe { ex.zend_call_arg(1usize) };
                                            let Some(__zval_1) = __zval_1 else {
                                                return;
                                            };
                                            match ::ext_php_rs::internal::argument::extract::<
                                                Option<&Counter>,
                                                _,
                                            >(__zval_1.dereference_mut(), &mut __holder_source) {
                                                Ok(value) => value,
                                                Err(error) => {
                                                    ::ext_php_rs::internal::argument::argument_error(
                                                            "source",
                                                            &error,
                                                        )
                                                        .throw();
                                                    return;
                                                }
                                            }
                                        } else {
                                            match ::ext_php_rs::internal::argument::missing::<
                                                Option<&Counter>,
                                                _,
                                            >() {
                                                Some(value) => value,
                                                None => {
                                                    ::ext_php_rs::exception::PhpException::new(
                                                            "Invalid value given for argument `source`.".into(),
                                                            0,
                                                            ::ext_php_rs::zend::ce::type_error(),
                                                        )
                                                        .throw();
                                                    return;
                                                }
                                            }
                                        };
                                        let __result = { merge(target, source) };
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
                        ::ext_php_rs::internal::argument::arg::<
                            &mut Counter,
                            _,
                        >("target"),
                    )
                    .arg(
                        ::ext_php_rs::internal::argument::arg::<
                            Option<&Counter>,
                            _,
                        >("source"),
                    )
                    .required_args(__REQUIRED)
                    .returns(::ext_php_rs::flags::DataType::Void, false, false)
            }
        }
        entry
    };
}
