use crate::{
    args::{Arg, ArgInfo},
    describe::DocComments,
    error::{Error, Result},
    flags::{DataType, MethodFlags},
    types::{PhpType, Zval},
    zend::{ExecuteData, FunctionEntry, ZendType},
};
use std::{ffi::CString, mem, ptr};

/// Function representation in Rust.
#[cfg(not(windows))]
pub type FunctionHandler = extern "C" fn(execute_data: &mut ExecuteData, retval: &mut Zval);
#[cfg(windows)]
pub type FunctionHandler =
    extern "vectorcall" fn(execute_data: &mut ExecuteData, retval: &mut Zval);

/// Function representation in Rust using pointers.
#[cfg(not(windows))]
type FunctionPointerHandler = extern "C" fn(execute_data: *mut ExecuteData, retval: *mut Zval);
#[cfg(windows)]
type FunctionPointerHandler =
    extern "vectorcall" fn(execute_data: *mut ExecuteData, retval: *mut Zval);

/// Builder for registering a function in PHP.
#[must_use]
#[derive(Debug)]
pub struct FunctionBuilder<'a> {
    pub(crate) name: String,
    function: FunctionEntry,
    pub(crate) args: Vec<Arg<'a>>,
    n_req: Option<usize>,
    pub(crate) retval: Option<PhpType>,
    ret_as_ref: bool,
    pub(crate) ret_as_null: bool,
    pub(crate) docs: DocComments,
}

impl<'a> FunctionBuilder<'a> {
    /// Creates a new function builder, used to build functions
    /// to be exported to PHP.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the function.
    /// * `handler` - The handler to be called when the function is invoked from
    ///   PHP.
    pub fn new<T: Into<String>>(name: T, handler: FunctionHandler) -> Self {
        Self {
            name: name.into(),
            function: FunctionEntry {
                fname: ptr::null(),
                // SAFETY: `*mut T` and `&mut T` have the same ABI as long as `*mut T` is non-null,
                // aligned and pointing to a `T`. PHP guarantees that these conditions will be met.
                handler: Some(unsafe {
                    mem::transmute::<FunctionHandler, FunctionPointerHandler>(handler)
                }),
                arg_info: ptr::null(),
                num_args: 0,
                flags: 0, // TBD?
                #[cfg(php84)]
                doc_comment: ptr::null(),
                #[cfg(php84)]
                frameless_function_infos: ptr::null(),
            },
            args: vec![],
            n_req: None,
            retval: None,
            ret_as_ref: false,
            ret_as_null: false,
            docs: &[],
        }
    }

    /// Create a new function builder for an abstract function that can be used
    /// on an abstract class or an interface.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the function.
    pub fn new_abstract<T: Into<String>>(name: T) -> Self {
        Self {
            name: name.into(),
            function: FunctionEntry {
                fname: ptr::null(),
                handler: None,
                arg_info: ptr::null(),
                num_args: 0,
                flags: MethodFlags::Abstract.bits(),
                #[cfg(php84)]
                doc_comment: ptr::null(),
                #[cfg(php84)]
                frameless_function_infos: ptr::null(),
            },
            args: vec![],
            n_req: None,
            retval: None,
            ret_as_ref: false,
            ret_as_null: false,
            docs: &[],
        }
    }

    /// Creates a constructor builder, used to build the constructor
    /// for classes.
    ///
    /// # Parameters
    ///
    /// * `handler` - The handler to be called when the function is invoked from
    ///   PHP.
    pub fn constructor(handler: FunctionHandler) -> Self {
        Self::new("__construct", handler)
    }

    /// Adds an argument to the function.
    ///
    /// # Parameters
    ///
    /// * `arg` - The argument to add to the function.
    pub fn arg(mut self, arg: Arg<'a>) -> Self {
        self.args.push(arg);
        self
    }

    /// Sets the rest of the given arguments as not required.
    pub fn not_required(mut self) -> Self {
        self.n_req = Some(self.args.len());
        self
    }

    /// Sets the return value of the function.
    ///
    /// Accepts a [`DataType`] for the simple case (via [`From<DataType> for
    /// PhpType`]) or a full [`PhpType`] for compound forms such as
    /// [`PhpType::Union`].
    ///
    /// # Parameters
    ///
    /// * `ty` - The return type of the function.
    /// * `as_ref` - Whether the function returns a reference.
    /// * `allow_null` - Whether the function return value is nullable.
    pub fn returns<T: Into<PhpType>>(mut self, ty: T, as_ref: bool, allow_null: bool) -> Self {
        let ty = ty.into();
        // PHP rejects `?void` and `?mixed`, so the nullable flag is squashed
        // for those single-type returns. Unions never resolve to those types
        // syntactically, so the user's `allow_null` is honoured directly.
        self.ret_as_null = match &ty {
            PhpType::Simple(dt) => {
                allow_null && *dt != DataType::Void && *dt != DataType::Mixed
            }
            PhpType::Union(_) => allow_null,
        };
        self.retval = Some(ty);
        self.ret_as_ref = as_ref;
        self
    }

    /// Sets the documentation for the function.
    /// This is used to generate the PHP stubs for the function.
    ///
    /// # Parameters
    ///
    /// * `docs` - The documentation for the function.
    pub fn docs(mut self, docs: DocComments) -> Self {
        self.docs = docs;
        self
    }

    /// Builds the function converting it into a Zend function entry.
    ///
    /// Returns a result containing the function entry if successful.
    ///
    /// # Errors
    ///
    /// * `Error::InvalidCString` - If the function name is not a valid C
    ///   string.
    /// * `Error::IntegerOverflow` - If the number of arguments is too large.
    /// * If arg info for an argument could not be created.
    /// * If the function name contains NUL bytes.
    pub fn build(mut self) -> Result<FunctionEntry> {
        let mut args = Vec::with_capacity(self.args.len() + 1);
        let mut n_req = self.n_req.unwrap_or(self.args.len());
        let variadic = self.args.last().is_some_and(|arg| arg.variadic);

        if variadic {
            self.function.flags |= MethodFlags::Variadic.bits();
            n_req = n_req.saturating_sub(1);
        }

        // argument header, retval etc
        // The first argument is used as `zend_internal_function_info` for the function.
        // That struct shares the same memory as `zend_internal_arg_info` which is used
        // for the arguments.
        args.push(ArgInfo {
            // required_num_args
            name: n_req as *const _,
            type_: match &self.retval {
                Some(PhpType::Simple(dt)) => ZendType::empty_from_type(
                    *dt,
                    self.ret_as_ref,
                    false,
                    self.ret_as_null,
                )
                .ok_or(Error::InvalidCString)?,
                Some(PhpType::Union(types)) => ZendType::empty_from_primitive_union(
                    types,
                    self.ret_as_ref,
                    false,
                    self.ret_as_null,
                )
                .ok_or(Error::InvalidCString)?,
                None => ZendType::empty(false, false),
            },
            default_value: ptr::null(),
        });

        // arguments
        args.extend(
            self.args
                .iter()
                .map(Arg::as_arg_info)
                .collect::<Result<Vec<_>>>()?,
        );

        self.function.fname = CString::new(self.name)?.into_raw();
        self.function.num_args = (args.len() - 1).try_into()?;
        self.function.arg_info = Box::into_raw(args.into_boxed_slice()) as *const ArgInfo;

        Ok(self.function)
    }
}
