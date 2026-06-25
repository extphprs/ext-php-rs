//! Builder and objects relating to function and method arguments.

use std::{ffi::CString, ptr};

use crate::{
    convert::{FromZvalMut, IntoZvalDyn},
    describe::{Parameter, abi},
    error::{Error, Result},
    ffi::{zend_internal_arg_info, zend_wrong_parameters_count_error},
    types::{PhpType, Zval},
    zend::ZendType,
};

/// Represents an argument to a function.
#[must_use]
#[derive(Debug)]
pub struct Arg<'a> {
    name: String,
    r#type: PhpType,
    as_ref: bool,
    allow_null: bool,
    pub(crate) variadic: bool,
    default_value: Option<String>,
    zval: Option<&'a mut Zval>,
    variadic_zvals: Vec<Option<&'a mut Zval>>,
}

impl<'a> Arg<'a> {
    /// Creates a new argument.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the parameter.
    /// * `ty` - The type of the parameter. Accepts a
    ///   [`crate::flags::DataType`] for the single-type case (via
    ///   [`From<crate::flags::DataType> for PhpType`]) or a full [`PhpType`]
    ///   for compound forms such as [`PhpType::Union`].
    pub fn new<T, U>(name: T, ty: U) -> Self
    where
        T: Into<String>,
        U: Into<PhpType>,
    {
        Arg {
            name: name.into(),
            r#type: ty.into(),
            as_ref: false,
            allow_null: false,
            variadic: false,
            default_value: None,
            zval: None,
            variadic_zvals: vec![],
        }
    }

    /// Sets the argument as a reference.
    #[allow(clippy::wrong_self_convention)]
    pub fn as_ref(mut self) -> Self {
        self.as_ref = true;
        self
    }

    /// Sets the argument as variadic.
    pub fn is_variadic(mut self) -> Self {
        self.variadic = true;
        self
    }

    /// Sets the argument as nullable.
    pub fn allow_null(mut self) -> Self {
        self.allow_null = true;
        self
    }

    /// Sets the default value for the argument.
    pub fn default<T: Into<String>>(mut self, default: T) -> Self {
        self.default_value = Some(default.into());
        self
    }

    /// Attempts to consume the argument, converting the inner type into `T`.
    /// Upon success, the result is returned in a [`Result`].
    ///
    /// As this function consumes, it cannot return a reference to the
    /// underlying zval.
    ///
    /// # Errors
    ///
    /// If the conversion fails (or the argument contains no value), the
    /// argument is returned in an [`Err`] variant.
    pub fn consume<T>(mut self) -> Result<T, Self>
    where
        for<'b> T: FromZvalMut<'b>,
    {
        self.zval
            .as_mut()
            .and_then(|zv| T::from_zval_mut(zv.dereference_mut()))
            .ok_or(self)
    }

    /// Attempts to retrieve the value of the argument.
    /// This will be None until the [`ArgParser`] is used to parse
    /// the arguments.
    pub fn val<T>(&'a mut self) -> Option<T>
    where
        T: FromZvalMut<'a>,
    {
        self.zval
            .as_mut()
            .and_then(|zv| T::from_zval_mut(zv.dereference_mut()))
    }

    /// Retrice all the variadic values for this Rust argument.
    pub fn variadic_vals<T>(&'a mut self) -> Vec<T>
    where
        T: FromZvalMut<'a>,
    {
        self.variadic_zvals
            .iter_mut()
            .filter_map(|zv| zv.as_mut())
            .filter_map(|zv| T::from_zval_mut(zv.dereference_mut()))
            .collect()
    }

    /// Attempts to return a reference to the arguments internal Zval.
    ///
    /// # Returns
    ///
    /// * `Some(&Zval)` - The internal zval.
    /// * `None` - The argument was empty.
    // TODO: Figure out if we can change this
    #[allow(clippy::mut_mut)]
    pub fn zval(&mut self) -> Option<&mut &'a mut Zval> {
        self.zval.as_mut()
    }

    /// Attempts to call the argument as a callable with a list of arguments to
    /// pass to the function. Note that a thrown exception inside the
    /// callable is not detectable, therefore you should check if the return
    /// value is valid rather than unwrapping. Returns a result containing the
    /// return value of the function, or an error.
    ///
    /// You should not call this function directly, rather through the
    /// [`call_user_func`](crate::call_user_func) macro.
    ///
    /// # Parameters
    ///
    /// * `params` - A list of parameters to call the function with.
    ///
    /// # Errors
    ///
    /// * `Error::Callable` - The argument is not callable.
    // TODO: Measure this
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub fn try_call(&self, params: Vec<&dyn IntoZvalDyn>) -> Result<Zval> {
        self.zval.as_ref().ok_or(Error::Callable)?.try_call(params)
    }

    /// Returns the legacy `Z_EXPECTED_*` discriminant for this argument.
    ///
    /// This is a thin projection used by extensions that drive PHP's legacy
    /// ZPP error path (`zend_wrong_parameter_type_error`) themselves. The
    /// discriminant enum predates compound types: PHP itself uses
    /// `zend_argument_type_error` with a custom format string for unions and
    /// intersections (see `Zend/zend_API.c` and `ext/standard/array.c`).
    ///
    /// For compound declared types, format the type via [`Arg::ty`] and
    /// throw a [`crate::exception::PhpException`] instead.
    ///
    /// # Errors
    ///
    /// * [`Error::NoExpectedTypeDiscriminant`] - the argument's declared
    ///   type has no equivalent in PHP's `Z_EXPECTED_*` enum (compound
    ///   types or scalar [`crate::flags::DataType`] variants without a slot,
    ///   such as `Mixed`, `Void`, `Iterable`, `Callable`, `Null`).
    pub fn expected_type(&self) -> Result<crate::zend::ExpectedType> {
        let dt = match &self.r#type {
            PhpType::Simple(dt) => *dt,
            _ => return Err(Error::NoExpectedTypeDiscriminant),
        };
        crate::zend::ExpectedType::from_simple(dt, self.allow_null)
            .ok_or(Error::NoExpectedTypeDiscriminant)
    }

    /// Returns the declared PHP type for this argument.
    ///
    /// Use [`std::fmt::Display`] on the result (e.g.
    /// `format!("{}", arg.ty())`) to render the canonical PHP-syntax
    /// string for the type, including unions, intersections, and DNF.
    #[must_use]
    pub fn ty(&self) -> &PhpType {
        &self.r#type
    }

    /// Returns the internal PHP argument info.
    pub(crate) fn as_arg_info(&self) -> Result<ArgInfo> {
        let zend_type = match &self.r#type {
            PhpType::Simple(dt) => {
                ZendType::empty_from_type(*dt, self.as_ref, self.variadic, self.allow_null)
                    .ok_or(Error::InvalidCString)?
            }
            PhpType::Union(types) => ZendType::empty_from_primitive_union(
                types,
                self.as_ref,
                self.variadic,
                self.allow_null,
            )
            .ok_or(Error::InvalidCString)?,
            PhpType::ClassUnion(class_names) => ZendType::empty_from_class_union(
                class_names,
                self.as_ref,
                self.variadic,
                self.allow_null,
            )
            .ok_or(Error::InvalidCString)?,
            #[cfg(php83)]
            PhpType::Intersection(class_names) => ZendType::empty_from_class_intersection(
                class_names,
                self.as_ref,
                self.variadic,
                self.allow_null,
            )
            .ok_or(Error::InvalidCString)?,
            #[cfg(not(php83))]
            PhpType::Intersection(_) => return Err(Error::InvalidCString),
            #[cfg(php83)]
            PhpType::Dnf(terms) => {
                ZendType::empty_from_dnf(terms, self.as_ref, self.variadic, self.allow_null)
                    .ok_or(Error::InvalidCString)?
            }
            #[cfg(not(php83))]
            PhpType::Dnf(_) => return Err(Error::InvalidCString),
        };
        Ok(ArgInfo {
            name: CString::new(self.name.as_str())?.into_raw(),
            type_: zend_type,
            default_value: match &self.default_value {
                Some(val) if val.as_str() == "None" => CString::new("null")?.into_raw(),
                Some(val) => CString::new(val.as_str())?.into_raw(),
                None => ptr::null(),
            },
        })
    }
}

impl From<Arg<'_>> for Parameter {
    fn from(val: Arg<'_>) -> Self {
        Parameter {
            name: val.name.into(),
            ty: Some(val.r#type.into()).into(),
            nullable: val.allow_null,
            variadic: val.variadic,
            default: val.default_value.map(abi::RString::from).into(),
        }
    }
}

/// Internal argument information used by Zend.
pub type ArgInfo = zend_internal_arg_info;

/// Parses the arguments of a function.
#[must_use]
pub struct ArgParser<'a, 'b> {
    args: Vec<&'b mut Arg<'a>>,
    min_num_args: Option<usize>,
    arg_zvals: Vec<Option<&'a mut Zval>>,
}

impl<'a, 'b> ArgParser<'a, 'b> {
    /// Builds a new function argument parser.
    pub fn new(arg_zvals: Vec<Option<&'a mut Zval>>) -> Self {
        ArgParser {
            args: vec![],
            min_num_args: None,
            arg_zvals,
        }
    }

    /// Adds a new argument to the parser.
    ///
    /// # Parameters
    ///
    /// * `arg` - The argument to add to the parser.
    pub fn arg(mut self, arg: &'b mut Arg<'a>) -> Self {
        self.args.push(arg);
        self
    }

    /// Sets the next arguments to be added as not required.
    pub fn not_required(mut self) -> Self {
        self.min_num_args = Some(self.args.len());
        self
    }

    /// Uses the argument parser to parse the arguments contained in the given
    /// `ExecuteData` object. Returns successfully if the arguments were
    /// parsed.
    ///
    /// This function can only be safely called from within an exported PHP
    /// function.
    ///
    /// # Parameters
    ///
    /// * `execute_data` - The execution data from the function.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] type if there were too many or too little arguments
    /// passed to the function. The user has already been notified so you
    /// should break execution after seeing an error type.
    ///
    /// Also returns an error if the number of min/max arguments exceeds
    /// `u32::MAX`
    pub fn parse(mut self) -> Result<()> {
        let max_num_args = self.args.len();
        let mut min_num_args = self.min_num_args.unwrap_or(max_num_args);
        let num_args = self.arg_zvals.len();
        let has_variadic = self.args.last().is_some_and(|arg| arg.variadic);
        if has_variadic {
            min_num_args = min_num_args.saturating_sub(1);
        }

        if num_args < min_num_args || (!has_variadic && num_args > max_num_args) {
            // SAFETY: Exported C function is safe, return value is unused and parameters
            // are copied.
            unsafe {
                zend_wrong_parameters_count_error(
                    min_num_args.try_into()?,
                    max_num_args.try_into()?,
                );
            };
            return Err(Error::IncorrectArguments(num_args, min_num_args));
        }

        for (i, arg_zval) in self.arg_zvals.into_iter().enumerate() {
            let arg = match self.args.get_mut(i) {
                Some(arg) => Some(arg),
                // Only select the last item if it's variadic
                None => self.args.last_mut().filter(|arg| arg.variadic),
            };
            if let Some(arg) = arg {
                if arg.variadic {
                    arg.variadic_zvals.push(arg_zval);
                } else {
                    arg.zval = arg_zval;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #[cfg(feature = "embed")]
    use crate::embed::Embed;
    use crate::flags::DataType;

    use super::*;

    #[test]
    fn test_new() {
        let arg = Arg::new("test", DataType::Long);
        assert_eq!(arg.name, "test");
        assert_eq!(arg.r#type, PhpType::Simple(DataType::Long));
        assert!(!arg.as_ref);
        assert!(!arg.allow_null);
        assert!(!arg.variadic);
        assert!(arg.default_value.is_none());
        assert!(arg.zval.is_none());
        assert!(arg.variadic_zvals.is_empty());
    }

    #[test]
    fn test_new_with_union() {
        let arg = Arg::new(
            "test",
            PhpType::Union(vec![DataType::Long, DataType::String]),
        );
        assert_eq!(
            arg.r#type,
            PhpType::Union(vec![DataType::Long, DataType::String])
        );
    }

    #[test]
    fn test_as_ref() {
        let arg = Arg::new("test", DataType::Long).as_ref();
        assert!(arg.as_ref);
    }

    #[test]
    fn test_is_variadic() {
        let arg = Arg::new("test", DataType::Long).is_variadic();
        assert!(arg.variadic);
    }

    #[test]
    fn test_allow_null() {
        let arg = Arg::new("test", DataType::Long).allow_null();
        assert!(arg.allow_null);
    }

    #[test]
    fn test_default() {
        let arg = Arg::new("test", DataType::Long).default("default");
        assert_eq!(arg.default_value, Some("default".to_string()));

        // TODO: Validate type
    }

    #[test]
    fn test_consume_no_value() {
        let arg = Arg::new("test", DataType::Long);
        let result: Result<i32, _> = arg.consume();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().name, "test");
    }

    #[test]
    #[cfg(feature = "embed")]
    fn test_consume() {
        let mut arg = Arg::new("test", DataType::Long);
        let mut zval = Zval::from(42);
        arg.zval = Some(&mut zval);

        let result: Result<i32, _> = arg.consume();
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_val_no_value() {
        let mut arg = Arg::new("test", DataType::Long);
        let result: Option<i32> = arg.val();
        assert!(result.is_none());
    }

    #[test]
    #[cfg(feature = "embed")]
    fn test_val() {
        let mut arg = Arg::new("test", DataType::Long);
        let mut zval = Zval::from(42);
        arg.zval = Some(&mut zval);

        let result: Option<i32> = arg.val();
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    #[cfg(feature = "embed")]
    fn test_variadic_vals() {
        let mut arg = Arg::new("test", DataType::Long).is_variadic();
        let mut zval1 = Zval::from(42);
        let mut zval2 = Zval::from(43);
        arg.variadic_zvals.push(Some(&mut zval1));
        arg.variadic_zvals.push(Some(&mut zval2));

        let result: Vec<i32> = arg.variadic_vals();
        assert_eq!(result, vec![42, 43]);
    }

    #[test]
    fn test_zval_no_value() {
        let mut arg = Arg::new("test", DataType::Long);
        let result = arg.zval();
        assert!(result.is_none());
    }

    #[test]
    #[cfg(feature = "embed")]
    fn test_zval() {
        let mut arg = Arg::new("test", DataType::Long);
        let mut zval = Zval::from(42);
        arg.zval = Some(&mut zval);

        let result = arg.zval();
        assert!(result.is_some());
        assert_eq!(result.unwrap().dereference_mut().long(), Some(42));
    }

    #[cfg(feature = "embed")]
    #[test]
    fn test_try_call_no_value() {
        let arg = Arg::new("test", DataType::Long);
        let result = arg.try_call(vec![]);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "embed")]
    fn test_try_call_not_callable() {
        Embed::run(|| {
            let mut arg = Arg::new("test", DataType::Long);
            let mut zval = Zval::from(42);
            arg.zval = Some(&mut zval);

            let result = arg.try_call(vec![]);
            assert!(result.is_err());
        });
    }

    // TODO: Test the callable case

    #[test]
    #[cfg(feature = "embed")]
    fn test_as_arg_info() {
        let arg = Arg::new("test", DataType::Long);
        let arg_info = arg.as_arg_info();
        assert!(arg_info.is_ok());

        let arg_info = arg_info.unwrap();
        assert!(arg_info.default_value.is_null());

        let r#type = arg_info.type_;
        assert_eq!(r#type.type_mask, 16);
    }

    #[test]
    #[cfg(feature = "embed")]
    fn test_as_arg_info_with_default() {
        let arg = Arg::new("test", DataType::Long).default("default");
        let arg_info = arg.as_arg_info();
        assert!(arg_info.is_ok());

        let arg_info = arg_info.unwrap();
        assert!(!arg_info.default_value.is_null());

        let r#type = arg_info.type_;
        assert_eq!(r#type.type_mask, 16);
    }

    #[test]
    fn test_param_from_arg() {
        let arg = Arg::new("test", DataType::Long)
            .default("default")
            .allow_null();
        let param: Parameter = arg.into();
        assert_eq!(param.name, "test".into());
        assert_eq!(
            param.ty,
            abi::Option::Some(crate::describe::PhpTypeAbi::Simple(DataType::Long))
        );
        assert!(param.nullable);
        assert_eq!(param.default, abi::Option::Some("default".into()));
    }

    #[test]
    fn test_arg_parser_new() {
        let arg_zvals = vec![None, None];
        let parser = ArgParser::new(arg_zvals);
        assert_eq!(parser.arg_zvals.len(), 2);
        assert!(parser.args.is_empty());
        assert!(parser.min_num_args.is_none());
    }

    #[test]
    fn test_arg_parser_arg() {
        let arg_zvals = vec![None, None];
        let mut parser = ArgParser::new(arg_zvals);
        let mut arg = Arg::new("test", DataType::Long);
        parser = parser.arg(&mut arg);
        assert_eq!(parser.args.len(), 1);
        assert_eq!(parser.args[0].name, "test");
        assert_eq!(parser.args[0].r#type, PhpType::Simple(DataType::Long));
    }

    #[test]
    #[cfg(php83)]
    fn class_union_arg_emits_literal_name_with_pipe_joined_classes() {
        use crate::ffi::_ZEND_TYPE_LITERAL_NAME_BIT;
        use std::ffi::CStr;

        let arg = Arg::new(
            "value",
            PhpType::ClassUnion(vec!["Foo".to_owned(), "Bar".to_owned()]),
        );
        let arg_info = arg.as_arg_info().expect("class union should build");

        assert_ne!(
            arg_info.type_.type_mask & _ZEND_TYPE_LITERAL_NAME_BIT,
            0,
            "literal-name bit must be set on PHP 8.3+",
        );
        assert!(!arg_info.type_.ptr.is_null());

        let class_str = unsafe { CStr::from_ptr(arg_info.type_.ptr.cast()) };
        assert_eq!(class_str.to_str().unwrap(), "Foo|Bar");
    }

    #[test]
    #[cfg(php83)]
    fn class_union_arg_with_allow_null_sets_nullable_bit() {
        use crate::ffi::_ZEND_TYPE_NULLABLE_BIT;

        let arg = Arg::new(
            "value",
            PhpType::ClassUnion(vec!["Foo".to_owned(), "Bar".to_owned()]),
        )
        .allow_null();
        let arg_info = arg
            .as_arg_info()
            .expect("nullable class union should build");

        assert_ne!(
            arg_info.type_.type_mask & _ZEND_TYPE_NULLABLE_BIT,
            0,
            "allow_null must propagate _ZEND_TYPE_NULLABLE_BIT",
        );
    }

    #[test]
    fn class_union_arg_with_empty_member_list_errors() {
        let arg = Arg::new("value", PhpType::ClassUnion(vec![]));
        assert!(arg.as_arg_info().is_err());
    }

    #[test]
    #[cfg(php83)]
    fn intersection_arg_emits_list_with_intersection_bit() {
        use crate::ffi::{_ZEND_TYPE_INTERSECTION_BIT, _ZEND_TYPE_LIST_BIT};

        let arg = Arg::new(
            "value",
            PhpType::Intersection(vec!["Countable".to_owned(), "Traversable".to_owned()]),
        );
        let arg_info = arg.as_arg_info().expect("intersection should build");

        assert_ne!(arg_info.type_.type_mask & _ZEND_TYPE_LIST_BIT, 0);
        assert_ne!(arg_info.type_.type_mask & _ZEND_TYPE_INTERSECTION_BIT, 0);
        assert!(!arg_info.type_.ptr.is_null());
    }

    #[test]
    #[cfg(php83)]
    fn intersection_arg_with_allow_null_errors() {
        let arg = Arg::new(
            "value",
            PhpType::Intersection(vec!["Foo".to_owned(), "Bar".to_owned()]),
        )
        .allow_null();
        assert!(
            arg.as_arg_info().is_err(),
            "nullable intersection must error: DNF lands in slice 04"
        );
    }

    #[test]
    #[cfg(php83)]
    fn intersection_arg_with_empty_member_list_errors() {
        let arg = Arg::new("value", PhpType::Intersection(vec![]));
        assert!(arg.as_arg_info().is_err());
    }

    #[test]
    #[cfg(php83)]
    fn dnf_arg_emits_outer_list_with_union_arena_bits() {
        use crate::ffi::{_ZEND_TYPE_ARENA_BIT, _ZEND_TYPE_LIST_BIT, _ZEND_TYPE_UNION_BIT};
        use crate::types::DnfTerm;

        let arg = Arg::new(
            "value",
            PhpType::Dnf(vec![
                DnfTerm::Intersection(vec!["A".to_owned(), "B".to_owned()]),
                DnfTerm::Single("C".to_owned()),
            ]),
        );
        let arg_info = arg.as_arg_info().expect("DNF arg should build");

        assert_ne!(arg_info.type_.type_mask & _ZEND_TYPE_LIST_BIT, 0);
        assert_ne!(arg_info.type_.type_mask & _ZEND_TYPE_UNION_BIT, 0);
        assert_ne!(arg_info.type_.type_mask & _ZEND_TYPE_ARENA_BIT, 0);
        assert!(!arg_info.type_.ptr.is_null());
    }

    #[test]
    #[cfg(php83)]
    fn dnf_arg_with_allow_null_sets_nullable_bit() {
        use crate::ffi::_ZEND_TYPE_NULLABLE_BIT;
        use crate::types::DnfTerm;

        let arg = Arg::new(
            "value",
            PhpType::Dnf(vec![
                DnfTerm::Intersection(vec!["A".to_owned(), "B".to_owned()]),
                DnfTerm::Single("C".to_owned()),
            ]),
        )
        .allow_null();
        let arg_info = arg.as_arg_info().expect("nullable DNF arg should build");

        assert_ne!(arg_info.type_.type_mask & _ZEND_TYPE_NULLABLE_BIT, 0);
    }

    #[test]
    #[cfg(php83)]
    fn dnf_arg_empty_terms_errors() {
        let arg = Arg::new("value", PhpType::Dnf(vec![]));
        assert!(arg.as_arg_info().is_err());
    }

    // TODO: test parse

    #[test]
    fn expected_type_for_simple_long() {
        let arg = Arg::new("v", DataType::Long);
        let got = arg.expected_type().expect("simple long should map");
        assert_eq!(got, crate::zend::ExpectedType::Long);
    }

    #[test]
    fn expected_type_for_nullable_simple_long() {
        let arg = Arg::new("v", DataType::Long).allow_null();
        let got = arg.expected_type().expect("nullable long should map");
        assert_eq!(got, crate::zend::ExpectedType::LongOrNull);
    }

    #[test]
    fn expected_type_for_simple_object() {
        let arg = Arg::new("v", DataType::Object(Some("Foo")));
        let got = arg.expected_type().expect("simple object should map");
        assert_eq!(got, crate::zend::ExpectedType::Object);
    }

    #[test]
    fn expected_type_for_nullable_object() {
        let arg = Arg::new("v", DataType::Object(None)).allow_null();
        let got = arg.expected_type().expect("nullable object should map");
        assert_eq!(got, crate::zend::ExpectedType::ObjectOrNull);
    }

    #[test]
    fn expected_type_for_unmappable_simple_returns_no_discriminant() {
        let arg = Arg::new("v", DataType::Mixed);
        assert!(matches!(
            arg.expected_type(),
            Err(Error::NoExpectedTypeDiscriminant)
        ));
    }

    #[test]
    fn expected_type_for_primitive_union_returns_no_discriminant() {
        let arg = Arg::new("v", PhpType::Union(vec![DataType::Long, DataType::String]));
        assert!(matches!(
            arg.expected_type(),
            Err(Error::NoExpectedTypeDiscriminant)
        ));
    }

    #[test]
    fn expected_type_for_class_union_returns_no_discriminant() {
        let arg = Arg::new(
            "v",
            PhpType::ClassUnion(vec!["Foo".to_owned(), "Bar".to_owned()]),
        );
        assert!(matches!(
            arg.expected_type(),
            Err(Error::NoExpectedTypeDiscriminant)
        ));
    }

    #[test]
    fn expected_type_for_intersection_returns_no_discriminant() {
        let arg = Arg::new(
            "v",
            PhpType::Intersection(vec!["Countable".to_owned(), "Traversable".to_owned()]),
        );
        assert!(matches!(
            arg.expected_type(),
            Err(Error::NoExpectedTypeDiscriminant)
        ));
    }

    #[test]
    fn ty_returns_simple_php_type() {
        let arg = Arg::new("v", DataType::Long);
        assert_eq!(arg.ty(), &PhpType::Simple(DataType::Long));
    }

    #[test]
    fn ty_returns_union_php_type() {
        let arg = Arg::new("v", PhpType::Union(vec![DataType::Long, DataType::String]));
        assert_eq!(
            arg.ty(),
            &PhpType::Union(vec![DataType::Long, DataType::String])
        );
    }

    #[test]
    fn ty_renders_as_php_syntax_string() {
        let arg = Arg::new(
            "v",
            PhpType::ClassUnion(vec!["Foo".to_owned(), "Bar".to_owned()]),
        );
        assert_eq!(format!("{}", arg.ty()), "\\Foo|\\Bar");
    }

    #[test]
    fn expected_type_for_dnf_returns_no_discriminant() {
        use crate::types::DnfTerm;
        let arg = Arg::new(
            "v",
            PhpType::Dnf(vec![
                DnfTerm::Intersection(vec!["A".to_owned(), "B".to_owned()]),
                DnfTerm::Single("C".to_owned()),
            ]),
        );
        assert!(matches!(
            arg.expected_type(),
            Err(Error::NoExpectedTypeDiscriminant)
        ));
    }
}
