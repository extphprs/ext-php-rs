//! Builder and objects relating to function and method arguments.

use std::{ffi::CString, ptr};

use crate::{
    convert::{FromZvalMut, IntoZvalDyn},
    describe::{Parameter, abi},
    error::{Error, Result},
    ffi::{
        _zend_expected_type, _zend_expected_type_Z_EXPECTED_ARRAY,
        _zend_expected_type_Z_EXPECTED_BOOL, _zend_expected_type_Z_EXPECTED_DOUBLE,
        _zend_expected_type_Z_EXPECTED_LONG, _zend_expected_type_Z_EXPECTED_OBJECT,
        _zend_expected_type_Z_EXPECTED_RESOURCE, _zend_expected_type_Z_EXPECTED_STRING,
        zend_internal_arg_info, zend_wrong_parameters_count_error,
    },
    flags::DataType,
    types::Zval,
    zend::ZendType,
};

/// Represents a single element in a DNF type - either a simple class or an
/// intersection group.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeGroup {
    /// A single class/interface type: `ArrayAccess`
    Single(String),
    /// An intersection of class/interface types: `Countable&Traversable`
    Intersection(Vec<String>),
}

/// Represents the PHP type(s) for an argument.
#[derive(Debug, Clone, PartialEq)]
pub enum ArgType {
    /// A single type (e.g., `int`, `string`, `MyClass`)
    Single(DataType),
    /// A union of primitive types (e.g., `int|string|null`)
    /// Note: For unions containing class types, use `UnionClasses`.
    Union(Vec<DataType>),
    /// An intersection of class/interface types (e.g., `Countable&Traversable`)
    /// Only available in PHP 8.1+.
    Intersection(Vec<String>),
    /// A union of class/interface types (e.g., `Foo|Bar`)
    UnionClasses(Vec<String>),
    /// A DNF (Disjunctive Normal Form) type (e.g.,
    /// `(Countable&Traversable)|ArrayAccess`) Only available in PHP 8.2+.
    Dnf(Vec<TypeGroup>),
}

impl PartialEq<DataType> for ArgType {
    fn eq(&self, other: &DataType) -> bool {
        match self {
            ArgType::Single(dt) => dt == other,
            ArgType::Union(_)
            | ArgType::Intersection(_)
            | ArgType::UnionClasses(_)
            | ArgType::Dnf(_) => false,
        }
    }
}

impl From<DataType> for ArgType {
    fn from(dt: DataType) -> Self {
        ArgType::Single(dt)
    }
}

impl ArgType {
    /// Returns the primary [`DataType`] for this argument type.
    /// For complex types, returns Mixed as a fallback for runtime type
    /// checking.
    #[must_use]
    pub fn primary_type(&self) -> DataType {
        match self {
            ArgType::Single(dt) => *dt,
            ArgType::Union(_)
            | ArgType::Intersection(_)
            | ArgType::UnionClasses(_)
            | ArgType::Dnf(_) => DataType::Mixed,
        }
    }

    /// Returns true if this type allows null values.
    #[must_use]
    pub fn allows_null(&self) -> bool {
        match self {
            ArgType::Single(dt) => matches!(dt, DataType::Null),
            ArgType::Union(types) => types.iter().any(|t| matches!(t, DataType::Null)),
            // Intersection, class union, and DNF types cannot directly include null
            // (use allow_null() for nullable variants)
            ArgType::Intersection(_) | ArgType::UnionClasses(_) | ArgType::Dnf(_) => false,
        }
    }
}

/// Represents an argument to a function.
#[must_use]
#[derive(Debug)]
pub struct Arg<'a> {
    name: String,
    r#type: ArgType,
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
    /// * `_type` - The type of the parameter.
    pub fn new<T: Into<String>>(name: T, r#type: DataType) -> Self {
        Arg {
            name: name.into(),
            r#type: ArgType::Single(r#type),
            as_ref: false,
            allow_null: false,
            variadic: false,
            default_value: None,
            zval: None,
            variadic_zvals: vec![],
        }
    }

    /// Creates a new argument with a union type.
    ///
    /// This creates a PHP union type (e.g., `int|string`) for the argument.
    /// Only primitive types are currently supported in unions.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the parameter.
    /// * `types` - The types to include in the union.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use ext_php_rs::args::Arg;
    /// use ext_php_rs::flags::DataType;
    ///
    /// // Creates an argument with type `int|string`
    /// let arg = Arg::new_union("value", vec![DataType::Long, DataType::String]);
    ///
    /// // Creates an argument with type `int|string|null`
    /// let nullable_arg = Arg::new_union("value", vec![
    ///     DataType::Long,
    ///     DataType::String,
    ///     DataType::Null,
    /// ]);
    /// ```
    pub fn new_union<T: Into<String>>(name: T, types: Vec<DataType>) -> Self {
        Arg {
            name: name.into(),
            r#type: ArgType::Union(types),
            as_ref: false,
            allow_null: false,
            variadic: false,
            default_value: None,
            zval: None,
            variadic_zvals: vec![],
        }
    }

    /// Creates a new argument with an intersection type (PHP 8.1+).
    ///
    /// This creates a PHP intersection type (e.g., `Countable&Traversable`) for
    /// the argument. The value must implement ALL of the specified interfaces.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the parameter.
    /// * `class_names` - The class/interface names that form the intersection.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use ext_php_rs::args::Arg;
    ///
    /// // Creates an argument with type `Countable&Traversable`
    /// let arg = Arg::new_intersection("value", vec![
    ///     "Countable".to_string(),
    ///     "Traversable".to_string(),
    /// ]);
    /// ```
    pub fn new_intersection<T: Into<String>>(name: T, class_names: Vec<String>) -> Self {
        Arg {
            name: name.into(),
            r#type: ArgType::Intersection(class_names),
            as_ref: false,
            allow_null: false,
            variadic: false,
            default_value: None,
            zval: None,
            variadic_zvals: vec![],
        }
    }

    /// Creates a new argument with a union of class types (PHP 8.0+).
    ///
    /// This creates a PHP union type where each element is a class/interface
    /// (e.g., `Foo|Bar`). For primitive type unions, use [`Self::new_union`].
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the parameter.
    /// * `class_names` - The class/interface names that form the union.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use ext_php_rs::args::Arg;
    ///
    /// // Creates an argument with type `Iterator|IteratorAggregate`
    /// let arg = Arg::new_union_classes("value", vec![
    ///     "Iterator".to_string(),
    ///     "IteratorAggregate".to_string(),
    /// ]);
    /// ```
    pub fn new_union_classes<T: Into<String>>(name: T, class_names: Vec<String>) -> Self {
        Arg {
            name: name.into(),
            r#type: ArgType::UnionClasses(class_names),
            as_ref: false,
            allow_null: false,
            variadic: false,
            default_value: None,
            zval: None,
            variadic_zvals: vec![],
        }
    }

    /// Creates a new argument with a DNF (Disjunctive Normal Form) type (PHP
    /// 8.2+).
    ///
    /// DNF types allow combining intersection and union types, such as
    /// `(Countable&Traversable)|ArrayAccess`.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the parameter.
    /// * `groups` - Type groups using explicit `TypeGroup` variants.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use ext_php_rs::args::{Arg, TypeGroup};
    ///
    /// // Creates an argument with type `(Countable&Traversable)|ArrayAccess`
    /// let arg = Arg::new_dnf("value", vec![
    ///     TypeGroup::Intersection(vec!["Countable".to_string(), "Traversable".to_string()]),
    ///     TypeGroup::Single("ArrayAccess".to_string()),
    /// ]);
    /// ```
    pub fn new_dnf<T: Into<String>>(name: T, groups: Vec<TypeGroup>) -> Self {
        Arg {
            name: name.into(),
            r#type: ArgType::Dnf(groups),
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

    /// Returns the internal PHP argument info.
    ///
    /// Note: Intersection, class union, and DNF types for internal function
    /// parameters are only supported in PHP 8.3+. On earlier versions,
    /// these fall back to `mixed` type. See: <https://github.com/php/php-src/pull/11969>
    pub(crate) fn as_arg_info(&self) -> Result<ArgInfo> {
        let type_ = match &self.r#type {
            ArgType::Single(dt) => {
                ZendType::empty_from_type(*dt, self.as_ref, self.variadic, self.allow_null)
                    .ok_or(Error::InvalidCString)?
            }
            ArgType::Union(types) => {
                // Primitive union types (int|string|null etc.) work on all PHP 8.x versions
                ZendType::union_primitive(types, self.as_ref, self.variadic)
            }
            #[cfg(php83)]
            ArgType::Intersection(class_names) => {
                // Intersection types for internal functions require PHP 8.3+
                let names: Vec<&str> = class_names.iter().map(String::as_str).collect();
                ZendType::intersection(&names, self.as_ref, self.variadic)
                    .ok_or(Error::InvalidCString)?
            }
            #[cfg(not(php83))]
            ArgType::Intersection(_) => {
                // PHP < 8.3 doesn't support intersection types for internal functions.
                // Fall back to mixed type with allow_null handling.
                ZendType::empty_from_type(
                    DataType::Mixed,
                    self.as_ref,
                    self.variadic,
                    self.allow_null,
                )
                .ok_or(Error::InvalidCString)?
            }
            #[cfg(php83)]
            ArgType::UnionClasses(class_names) => {
                // Class union types for internal functions require PHP 8.3+
                let names: Vec<&str> = class_names.iter().map(String::as_str).collect();
                ZendType::union_classes(&names, self.as_ref, self.variadic, self.allow_null)
                    .ok_or(Error::InvalidCString)?
            }
            #[cfg(not(php83))]
            ArgType::UnionClasses(_) => {
                // PHP < 8.3 doesn't support class union types for internal functions.
                // Fall back to mixed type.
                ZendType::empty_from_type(
                    DataType::Mixed,
                    self.as_ref,
                    self.variadic,
                    self.allow_null,
                )
                .ok_or(Error::InvalidCString)?
            }
            #[cfg(php83)]
            ArgType::Dnf(groups) => {
                // DNF types for internal functions require PHP 8.3+
                let groups: Vec<Vec<&str>> = groups
                    .iter()
                    .map(|g| match g {
                        TypeGroup::Single(name) => vec![name.as_str()],
                        TypeGroup::Intersection(names) => {
                            names.iter().map(String::as_str).collect()
                        }
                    })
                    .collect();
                let groups_refs: Vec<&[&str]> = groups.iter().map(Vec::as_slice).collect();
                ZendType::dnf(&groups_refs, self.as_ref, self.variadic)
                    .ok_or(Error::InvalidCString)?
            }
            #[cfg(not(php83))]
            ArgType::Dnf(_) => {
                // PHP < 8.3 doesn't support DNF types for internal functions.
                // Fall back to mixed type.
                ZendType::empty_from_type(
                    DataType::Mixed,
                    self.as_ref,
                    self.variadic,
                    self.allow_null,
                )
                .ok_or(Error::InvalidCString)?
            }
        };

        Ok(ArgInfo {
            name: CString::new(self.name.as_str())?.into_raw(),
            type_,
            default_value: match &self.default_value {
                Some(val) if val.as_str() == "None" => CString::new("null")?.into_raw(),
                Some(val) => CString::new(val.as_str())?.into_raw(),
                None => ptr::null(),
            },
        })
    }
}

impl From<Arg<'_>> for _zend_expected_type {
    fn from(arg: Arg) -> Self {
        // For union types, we use the primary type for expected type errors
        let dt = arg.r#type.primary_type();
        let type_id = match dt {
            DataType::False | DataType::True => _zend_expected_type_Z_EXPECTED_BOOL,
            DataType::Long => _zend_expected_type_Z_EXPECTED_LONG,
            DataType::Double => _zend_expected_type_Z_EXPECTED_DOUBLE,
            DataType::String => _zend_expected_type_Z_EXPECTED_STRING,
            DataType::Array => _zend_expected_type_Z_EXPECTED_ARRAY,
            DataType::Resource => _zend_expected_type_Z_EXPECTED_RESOURCE,
            // Object, Mixed (used by unions), and other types use OBJECT as a fallback
            _ => _zend_expected_type_Z_EXPECTED_OBJECT,
        };

        if arg.allow_null { type_id + 1 } else { type_id }
    }
}

impl From<Arg<'_>> for Parameter {
    fn from(val: Arg<'_>) -> Self {
        // For Parameter (used in describe), use the primary type
        // TODO: Extend Parameter to support union/intersection/DNF types for better
        // stub generation
        let ty = match &val.r#type {
            ArgType::Single(dt) => Some(*dt),
            // For complex types, fall back to Mixed (Object would be more accurate for class types)
            ArgType::Union(_)
            | ArgType::Intersection(_)
            | ArgType::UnionClasses(_)
            | ArgType::Dnf(_) => Some(DataType::Mixed),
        };
        Parameter {
            name: val.name.into(),
            ty: ty.into(),
            nullable: val.allow_null || val.r#type.allows_null(),
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

    use super::*;

    #[test]
    fn test_new() {
        let arg = Arg::new("test", DataType::Long);
        assert_eq!(arg.name, "test");
        assert_eq!(arg.r#type, DataType::Long);
        assert!(!arg.as_ref);
        assert!(!arg.allow_null);
        assert!(!arg.variadic);
        assert!(arg.default_value.is_none());
        assert!(arg.zval.is_none());
        assert!(arg.variadic_zvals.is_empty());
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
    fn test_type_from_arg() {
        let arg = Arg::new("test", DataType::Long);
        let actual: _zend_expected_type = arg.into();
        assert_eq!(actual, 0);

        let arg = Arg::new("test", DataType::Long).allow_null();
        let actual: _zend_expected_type = arg.into();
        assert_eq!(actual, 1);

        let arg = Arg::new("test", DataType::False);
        let actual: _zend_expected_type = arg.into();
        assert_eq!(actual, 2);

        let arg = Arg::new("test", DataType::False).allow_null();
        let actual: _zend_expected_type = arg.into();
        assert_eq!(actual, 3);

        let arg = Arg::new("test", DataType::True);
        let actual: _zend_expected_type = arg.into();
        assert_eq!(actual, 2);

        let arg = Arg::new("test", DataType::True).allow_null();
        let actual: _zend_expected_type = arg.into();
        assert_eq!(actual, 3);

        let arg = Arg::new("test", DataType::String);
        let actual: _zend_expected_type = arg.into();
        assert_eq!(actual, 4);

        let arg = Arg::new("test", DataType::String).allow_null();
        let actual: _zend_expected_type = arg.into();
        assert_eq!(actual, 5);

        let arg = Arg::new("test", DataType::Array);
        let actual: _zend_expected_type = arg.into();
        assert_eq!(actual, 6);

        let arg = Arg::new("test", DataType::Array).allow_null();
        let actual: _zend_expected_type = arg.into();
        assert_eq!(actual, 7);

        let arg = Arg::new("test", DataType::Resource);
        let actual: _zend_expected_type = arg.into();
        assert_eq!(actual, 14);

        let arg = Arg::new("test", DataType::Resource).allow_null();
        let actual: _zend_expected_type = arg.into();
        assert_eq!(actual, 15);

        let arg = Arg::new("test", DataType::Object(None));
        let actual: _zend_expected_type = arg.into();
        assert_eq!(actual, 18);

        let arg = Arg::new("test", DataType::Object(None)).allow_null();
        let actual: _zend_expected_type = arg.into();
        assert_eq!(actual, 19);

        let arg = Arg::new("test", DataType::Double);
        let actual: _zend_expected_type = arg.into();
        assert_eq!(actual, 20);

        let arg = Arg::new("test", DataType::Double).allow_null();
        let actual: _zend_expected_type = arg.into();
        assert_eq!(actual, 21);
    }

    #[test]
    fn test_param_from_arg() {
        let arg = Arg::new("test", DataType::Long)
            .default("default")
            .allow_null();
        let param: Parameter = arg.into();
        assert_eq!(param.name, "test".into());
        assert_eq!(param.ty, abi::Option::Some(DataType::Long));
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
        assert_eq!(parser.args[0].r#type, DataType::Long);
    }

    #[test]
    fn test_new_union() {
        let arg = Arg::new_union("test", vec![DataType::Long, DataType::String]);
        assert_eq!(arg.name, "test");
        assert!(matches!(arg.r#type, ArgType::Union(_)));
        if let ArgType::Union(types) = &arg.r#type {
            assert_eq!(types.len(), 2);
            assert!(types.contains(&DataType::Long));
            assert!(types.contains(&DataType::String));
        }
        assert!(!arg.as_ref);
        assert!(!arg.allow_null);
        assert!(!arg.variadic);
    }

    #[test]
    fn test_union_with_null() {
        let arg = Arg::new_union(
            "nullable",
            vec![DataType::Long, DataType::String, DataType::Null],
        );
        assert!(arg.r#type.allows_null());
    }

    #[test]
    fn test_union_without_null() {
        let arg = Arg::new_union("non_nullable", vec![DataType::Long, DataType::String]);
        assert!(!arg.r#type.allows_null());
    }

    #[test]
    fn test_argtype_primary_type() {
        let single = ArgType::Single(DataType::Long);
        assert_eq!(single.primary_type(), DataType::Long);

        let union = ArgType::Union(vec![DataType::Long, DataType::String]);
        assert_eq!(union.primary_type(), DataType::Mixed);
    }

    #[test]
    fn test_argtype_eq_datatype() {
        let single = ArgType::Single(DataType::Long);
        assert_eq!(single, DataType::Long);
        assert_ne!(single, DataType::String);

        let union = ArgType::Union(vec![DataType::Long, DataType::String]);
        // Union should not equal any single DataType
        assert_ne!(union, DataType::Long);
        assert_ne!(union, DataType::Mixed);
    }

    // TODO: test parse
}
