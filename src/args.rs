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

/// Represents an argument to a function.
#[must_use]
#[derive(Debug)]
pub struct Arg<'a> {
    name: String,
    r#type: DataType,
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
            r#type,
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
    pub(crate) fn as_arg_info(&self) -> Result<ArgInfo> {
        Ok(ArgInfo {
            name: CString::new(self.name.as_str())?.into_raw(),
            type_: ZendType::empty_from_type(
                self.r#type,
                self.as_ref,
                self.variadic,
                self.allow_null,
            )
            .ok_or(Error::InvalidCString)?,
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
        let type_id = match arg.r#type {
            DataType::False | DataType::True => _zend_expected_type_Z_EXPECTED_BOOL,
            DataType::Long => _zend_expected_type_Z_EXPECTED_LONG,
            DataType::Double => _zend_expected_type_Z_EXPECTED_DOUBLE,
            DataType::String => _zend_expected_type_Z_EXPECTED_STRING,
            DataType::Array => _zend_expected_type_Z_EXPECTED_ARRAY,
            DataType::Object(_) => _zend_expected_type_Z_EXPECTED_OBJECT,
            DataType::Resource => _zend_expected_type_Z_EXPECTED_RESOURCE,
            _ => unreachable!(),
        };

        if arg.allow_null { type_id + 1 } else { type_id }
    }
}

impl From<Arg<'_>> for Parameter {
    fn from(val: Arg<'_>) -> Self {
        Parameter {
            name: val.name.into(),
            ty: Some(val.r#type).into(),
            nullable: val.allow_null,
            variadic: val.variadic,
            default: val.default_value.map(abi::RString::from).into(),
        }
    }
}

/// Internal argument information used by Zend.
pub type ArgInfo = zend_internal_arg_info;

/// Maximum number of arguments that can be stored on the stack.
/// Arguments beyond this count will be stored on the heap.
pub const MAX_STACK_ARGS: usize = 8;

/// Storage for zval arguments that avoids heap allocation for common cases.
///
/// For functions with <= 8 arguments, the zvals are stored directly on the
/// stack. For functions with more arguments, they are stored in a Vec on the
/// heap.
#[derive(Debug)]
pub enum ArgZvals<'a> {
    /// Stack-allocated storage for up to `MAX_STACK_ARGS` arguments.
    Stack {
        /// The array of arguments stored on the stack.
        args: [Option<&'a mut Zval>; MAX_STACK_ARGS],
        /// The actual number of arguments (may be less than array size).
        len: usize,
    },
    /// Heap-allocated storage for more than `MAX_STACK_ARGS` arguments.
    Heap(Vec<Option<&'a mut Zval>>),
}

impl<'a> ArgZvals<'a> {
    /// Creates a new stack-based storage from an iterator of zvals.
    /// Falls back to heap allocation if there are more than `MAX_STACK_ARGS`
    /// arguments.
    #[inline]
    #[allow(clippy::should_implement_trait)]
    pub fn from_iter<I: Iterator<Item = Option<&'a mut Zval>>>(mut iter: I) -> Self {
        let mut args: [Option<&'a mut Zval>; MAX_STACK_ARGS] = Default::default();
        let mut len = 0;

        // Fill the stack array
        for i in 0..MAX_STACK_ARGS {
            match iter.next() {
                Some(arg) => {
                    args[i] = arg;
                    len = i + 1;
                }
                None => {
                    // All arguments fit on the stack
                    return Self::Stack { args, len };
                }
            }
        }

        // Check if there are more arguments that don't fit on the stack
        if let Some(extra_arg) = iter.next() {
            // Exceeded stack capacity, convert to heap
            let mut vec: Vec<Option<&'a mut Zval>> = args.into_iter().collect();
            vec.push(extra_arg);
            vec.extend(iter);
            return Self::Heap(vec);
        }

        Self::Stack { args, len }
    }

    /// Returns the number of arguments.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Stack { len, .. } => *len,
            Self::Heap(vec) => vec.len(),
        }
    }

    /// Returns true if there are no arguments.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Converts self into an iterator over the arguments.
    #[inline]
    #[allow(clippy::should_implement_trait)]
    pub fn into_iter(self) -> impl Iterator<Item = Option<&'a mut Zval>> {
        ArgZvalsIter {
            inner: self,
            pos: 0,
        }
    }
}

/// Iterator over `ArgZvals`.
struct ArgZvalsIter<'a> {
    inner: ArgZvals<'a>,
    pos: usize,
}

impl<'a> Iterator for ArgZvalsIter<'a> {
    type Item = Option<&'a mut Zval>;

    fn next(&mut self) -> Option<Self::Item> {
        let len = self.inner.len();
        if self.pos >= len {
            return None;
        }
        let pos = self.pos;
        self.pos += 1;

        match &mut self.inner {
            ArgZvals::Stack { args, .. } => {
                // Take the value out and replace with None
                Some(args[pos].take())
            }
            ArgZvals::Heap(vec) => Some(vec[pos].take()),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.inner.len().saturating_sub(self.pos);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for ArgZvalsIter<'_> {}

/// Parses the arguments of a function.
#[must_use]
pub struct ArgParser<'a, 'b> {
    args: Vec<&'b mut Arg<'a>>,
    min_num_args: Option<usize>,
    arg_zvals: ArgZvals<'a>,
}

impl<'a, 'b> ArgParser<'a, 'b> {
    /// Builds a new function argument parser.
    ///
    /// Note: For better performance, prefer using [`from_arg_zvals`] which
    /// avoids heap allocation for functions with <= 8 arguments.
    ///
    /// [`from_arg_zvals`]: #method.from_arg_zvals
    pub fn new(arg_zvals: Vec<Option<&'a mut Zval>>) -> Self {
        ArgParser {
            args: vec![],
            min_num_args: None,
            arg_zvals: ArgZvals::Heap(arg_zvals),
        }
    }

    /// Builds a new function argument parser with optimized storage.
    ///
    /// This constructor uses stack-based storage for functions with <= 8
    /// arguments, avoiding heap allocation for the common case.
    #[inline]
    pub fn from_arg_zvals(arg_zvals: ArgZvals<'a>) -> Self {
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

        // Use the optimized iterator that works with both stack and heap storage
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

    // TODO: test parse

    // =========================================================================
    // ArgZvals tests - Stack-based argument storage
    // =========================================================================

    #[test]
    fn test_arg_zvals_empty() {
        let zvals: ArgZvals = ArgZvals::from_iter(std::iter::empty());
        assert!(zvals.is_empty());
        assert_eq!(zvals.len(), 0);
    }

    #[test]
    fn test_arg_zvals_stack_single() {
        let zvals = ArgZvals::from_iter([None::<&mut Zval>].into_iter());
        assert!(!zvals.is_empty());
        assert_eq!(zvals.len(), 1);
        matches!(zvals, ArgZvals::Stack { len: 1, .. });
    }

    #[test]
    fn test_arg_zvals_stack_max() {
        // Test with exactly MAX_STACK_ARGS elements
        let items: Vec<Option<&mut Zval>> = (0..MAX_STACK_ARGS).map(|_| None).collect();
        let zvals = ArgZvals::from_iter(items.into_iter());
        assert_eq!(zvals.len(), MAX_STACK_ARGS);
        matches!(zvals, ArgZvals::Stack { .. });
    }

    #[test]
    fn test_arg_zvals_heap_overflow() {
        // Test with more than MAX_STACK_ARGS elements - should use heap
        let items: Vec<Option<&mut Zval>> = (0..=MAX_STACK_ARGS).map(|_| None).collect();
        let zvals = ArgZvals::from_iter(items.into_iter());
        assert_eq!(zvals.len(), MAX_STACK_ARGS + 1);
        matches!(zvals, ArgZvals::Heap(_));
    }

    #[test]
    fn test_arg_zvals_heap_large() {
        // Test with many more elements
        let items: Vec<Option<&mut Zval>> = (0..20).map(|_| None).collect();
        let zvals = ArgZvals::from_iter(items.into_iter());
        assert_eq!(zvals.len(), 20);
        matches!(zvals, ArgZvals::Heap(_));
    }

    #[test]
    fn test_arg_zvals_into_iter_stack() {
        let items: Vec<Option<&mut Zval>> = (0..3).map(|_| None).collect();
        let zvals = ArgZvals::from_iter(items.into_iter());
        let collected: Vec<_> = zvals.into_iter().collect();
        assert_eq!(collected.len(), 3);
    }

    #[test]
    fn test_arg_zvals_into_iter_heap() {
        let items: Vec<Option<&mut Zval>> = (0..12).map(|_| None).collect();
        let zvals = ArgZvals::from_iter(items.into_iter());
        let collected: Vec<_> = zvals.into_iter().collect();
        assert_eq!(collected.len(), 12);
    }

    #[test]
    fn test_arg_parser_from_arg_zvals() {
        let items: Vec<Option<&mut Zval>> = (0..3).map(|_| None).collect();
        let zvals = ArgZvals::from_iter(items.into_iter());
        let parser = ArgParser::from_arg_zvals(zvals);
        assert_eq!(parser.arg_zvals.len(), 3);
        assert!(parser.args.is_empty());
    }
}
