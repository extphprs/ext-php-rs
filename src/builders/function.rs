use crate::{
    args::{Arg, ArgInfo},
    describe::DocComments,
    error::{Error, Result},
    flags::{DataType, MethodFlags},
    types::Zval,
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
    pub(crate) retval: Option<DataType>,
    /// The other members of a union return type, see
    /// [`FunctionBuilder::or_returns`].
    ret_union: Vec<DataType>,
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
            ret_union: Vec::new(),
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
        let mut function = FunctionEntry::end();
        function.add_flags(MethodFlags::Abstract);
        Self {
            name: name.into(),
            function,
            args: vec![],
            n_req: None,
            retval: None,
            ret_union: Vec::new(),
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

    /// The function's name and the zero-based offsets of its arguments marked
    /// [`sensitive`](Arg::sensitive), or `None` when it has none.
    pub(crate) fn sensitive(&self) -> Option<(String, Vec<u32>)> {
        let offsets: Vec<u32> = self
            .args
            .iter()
            .zip(0u32..)
            .filter(|(arg, _)| arg.sensitive)
            .map(|(_, offset)| offset)
            .collect();
        (!offsets.is_empty()).then(|| (self.name.clone(), offsets))
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

    /// Sets how many leading arguments are required, see
    /// [`required_count`](crate::args::required_count).
    pub fn required_args(mut self, count: usize) -> Self {
        self.n_req = Some(count);
        self
    }

    /// Adds `type_` to the return type set with [`FunctionBuilder::returns`],
    /// making it a union: `.returns(DataType::Long, false, false)
    /// .or_returns(DataType::Double)` declares `: int|float`. The types are
    /// those [`Arg::or`](crate::args::Arg::or) accepts; any other, or a union
    /// without [`FunctionBuilder::returns`], fails [`FunctionBuilder::build`]
    /// with [`Error::InvalidUnionType`].
    ///
    /// Debug builds of PHP check an internal function's return value against
    /// its declared type and assert on a mismatch: the handler must return a
    /// value of the union (`null` only when it holds `null`), including when
    /// it throws on bad input.
    pub fn or_returns(mut self, type_: DataType) -> Self {
        self.ret_union.push(type_);
        self
    }

    /// Sets the return value of the function.
    ///
    /// # Parameters
    ///
    /// * `type_` - The return type of the function.
    /// * `as_ref` - Whether the function returns a reference.
    /// * `allow_null` - Whether the function return value is nullable.
    pub fn returns(mut self, type_: DataType, as_ref: bool, allow_null: bool) -> Self {
        self.retval = Some(type_);
        self.ret_as_ref = as_ref;
        self.ret_as_null = allow_null && type_ != DataType::Void && type_ != DataType::Mixed;
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
    ///
    /// # Ownership
    ///
    /// The returned `Box<[ArgInfo]>` is what `FunctionEntry::arg_info` points at.
    /// The Zend engine borrows it for the life of the process, so the caller must
    /// park it somewhere that lives that long — see `ClassMetadata::arg_info` and
    /// `StaticModuleEntry`. Dropping it dangles the registered function.
    pub fn build(mut self) -> Result<(FunctionEntry, Box<[ArgInfo]>)> {
        let mut args = Vec::with_capacity(self.args.len() + 1);
        let mut n_req = self.n_req.unwrap_or(self.args.len());
        let variadic = self.args.last().is_some_and(|arg| arg.variadic);

        if variadic {
            self.function.add_flags(MethodFlags::Variadic);
            n_req = n_req.saturating_sub(1);
        }

        // argument header, retval etc
        // The first argument is used as `zend_internal_function_info` for the function.
        // That struct shares the same memory as `zend_internal_arg_info` which is used
        // for the arguments.
        args.push(ArgInfo {
            // required_num_args
            name: n_req as *const _,
            type_: match self.retval {
                Some(retval) => {
                    let mut type_ =
                        ZendType::empty_from_type(retval, self.ret_as_ref, false, self.ret_as_null)
                            .ok_or(Error::ZvalConversion(retval))?;
                    ZendType::add_union(
                        &mut type_,
                        &format!("{}()", self.name),
                        retval,
                        &self.ret_union,
                    )?;
                    type_
                }
                None if !self.ret_union.is_empty() => {
                    return Err(Error::InvalidUnionType(format!(
                        "{}(): or_returns without a return type to join",
                        self.name
                    )));
                }
                None => ZendType::empty(false, false),
            },
            default_value: ptr::null(),
        });

        // arguments
        args.extend(
            self.args
                .iter()
                .enumerate()
                .map(|(i, arg)| arg.as_arg_info(i >= n_req))
                .collect::<Result<Vec<_>>>()?,
        );

        self.function.fname = CString::new(self.name)?.into_raw();
        self.function.num_args = (args.len() - 1).try_into()?;

        let args = args.into_boxed_slice();
        self.function.arg_info = args.as_ptr();

        Ok((self.function, args))
    }
}

/// Frees a [`FunctionEntry`] table once the Zend engine has consumed it.
///
/// `zend_register_functions` interns every `fname`, and
/// `do_register_internal_class` reads `zend_class_entry.info.internal.builtin_functions`
/// exactly once and never frees it, so a class or enum method table is dead as
/// soon as registration returns.
///
/// The `arg_info` arrays are deliberately left alone. The engine's copy of them
/// is shallow, so `arg_info[i].name` and `.default_value` stay live for
/// `ReflectionParameter`, and a function with no parameters and no return type
/// keeps the array itself. They are owned by `ClassMetadata` or
/// [`ModuleAllocations`](crate::zend::ModuleAllocations) instead.
///
/// # Safety
///
/// * `entries` must come from `Box::into_raw` on a NUL-terminated
///   `Box<[FunctionEntry]>` whose entries were built by
///   [`FunctionBuilder::build`].
/// * Must be called exactly once, after the `zend_register_*` call that
///   consumed the table.
/// * Must never be called on a [`ModuleEntry::functions`] table: PHP re-reads
///   that one from `module_destructor` after MSHUTDOWN and from
///   `get_extension_funcs()` during a request.
///
/// [`ModuleEntry::functions`]: crate::zend::ModuleEntry
pub(crate) unsafe fn free_registered_entries(entries: *mut [FunctionEntry]) {
    let entries = unsafe { Box::from_raw(entries) };
    for entry in &*entries {
        if !entry.fname.is_null() {
            drop(unsafe { CString::from_raw(entry.fname.cast_mut()) });
        }
    }
}

#[cfg(test)]
mod union_tests {
    use super::*;
    use crate::test::test_function;

    #[test]
    fn or_returns_without_a_return_type_is_refused() {
        let built = FunctionBuilder::new("f", test_function)
            .or_returns(DataType::Long)
            .build();
        assert!(matches!(built, Err(Error::InvalidUnionType(message)) if message.contains("f()")));
    }

    #[test]
    fn a_union_return_type_ors_the_members() {
        let (entry, info) = FunctionBuilder::new("f", test_function)
            .returns(DataType::Long, false, false)
            .or_returns(DataType::Double)
            .build()
            .expect("a union of scalar types");
        // `build` hands the name to the engine; nothing registers it here.
        drop(unsafe { std::ffi::CString::from_raw(entry.fname.cast_mut()) });
        assert_eq!(
            info[0].type_.type_mask,
            (1 << crate::ffi::IS_LONG) | (1 << crate::ffi::IS_DOUBLE)
        );
    }
}
