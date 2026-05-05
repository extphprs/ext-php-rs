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
            PhpType::Simple(dt) => allow_null && *dt != DataType::Void && *dt != DataType::Mixed,
            PhpType::Union(_)
            | PhpType::ClassUnion(_)
            | PhpType::Intersection(_)
            | PhpType::Dnf(_) => allow_null,
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
                Some(PhpType::Simple(dt)) => {
                    ZendType::empty_from_type(*dt, self.ret_as_ref, false, self.ret_as_null)
                        .ok_or(Error::InvalidCString)?
                }
                Some(PhpType::Union(types)) => ZendType::empty_from_primitive_union(
                    types,
                    self.ret_as_ref,
                    false,
                    self.ret_as_null,
                )
                .ok_or(Error::InvalidCString)?,
                Some(PhpType::ClassUnion(class_names)) => ZendType::empty_from_class_union(
                    class_names,
                    self.ret_as_ref,
                    false,
                    self.ret_as_null,
                )
                .ok_or(Error::InvalidCString)?,
                #[cfg(php83)]
                Some(PhpType::Intersection(class_names)) => {
                    ZendType::empty_from_class_intersection(
                        class_names,
                        self.ret_as_ref,
                        false,
                        self.ret_as_null,
                    )
                    .ok_or(Error::InvalidCString)?
                }
                #[cfg(not(php83))]
                Some(PhpType::Intersection(_)) => return Err(Error::InvalidCString),
                #[cfg(php83)]
                Some(PhpType::Dnf(terms)) => {
                    ZendType::empty_from_dnf(terms, self.ret_as_ref, false, self.ret_as_null)
                        .ok_or(Error::InvalidCString)?
                }
                #[cfg(not(php83))]
                Some(PhpType::Dnf(_)) => return Err(Error::InvalidCString),
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[cfg(php83)]
    use crate::zend_fastcall;

    #[cfg(php83)]
    zend_fastcall! {
        extern "C" fn noop_handler(_: &mut ExecuteData, _: &mut Zval) {}
    }

    #[test]
    #[cfg(php83)]
    fn returns_class_union_emits_literal_name_on_retval_arg_info() {
        use crate::ffi::_ZEND_TYPE_LITERAL_NAME_BIT;
        use std::ffi::CStr;

        let entry = FunctionBuilder::new("ret_class_union", noop_handler)
            .returns(
                PhpType::ClassUnion(vec!["Foo".to_owned(), "Bar".to_owned()]),
                false,
                false,
            )
            .build()
            .expect("class union return should build");

        // arg_info[0] is the retval slot (zend_internal_function_info).
        let retval_info = unsafe { &*entry.arg_info };
        assert_ne!(retval_info.type_.type_mask & _ZEND_TYPE_LITERAL_NAME_BIT, 0,);
        assert!(!retval_info.type_.ptr.is_null());
        let class_str = unsafe { CStr::from_ptr(retval_info.type_.ptr.cast()) };
        assert_eq!(class_str.to_str().unwrap(), "Foo|Bar");
    }

    #[test]
    #[cfg(php83)]
    fn returns_class_union_with_allow_null_propagates_nullable_bit() {
        use crate::ffi::_ZEND_TYPE_NULLABLE_BIT;

        let entry = FunctionBuilder::new("ret_nullable_class_union", noop_handler)
            .returns(
                PhpType::ClassUnion(vec!["Foo".to_owned(), "Bar".to_owned()]),
                false,
                true,
            )
            .build()
            .expect("nullable class union return should build");

        let retval_info = unsafe { &*entry.arg_info };
        assert_ne!(retval_info.type_.type_mask & _ZEND_TYPE_NULLABLE_BIT, 0);
    }

    #[test]
    #[cfg(php83)]
    fn returns_intersection_emits_list_with_intersection_bit_on_retval() {
        use crate::ffi::{_ZEND_TYPE_INTERSECTION_BIT, _ZEND_TYPE_LIST_BIT};

        let entry = FunctionBuilder::new("ret_intersection", noop_handler)
            .returns(
                PhpType::Intersection(vec!["Countable".to_owned(), "Traversable".to_owned()]),
                false,
                false,
            )
            .build()
            .expect("intersection return should build");

        let retval_info = unsafe { &*entry.arg_info };
        assert_ne!(retval_info.type_.type_mask & _ZEND_TYPE_LIST_BIT, 0);
        assert_ne!(retval_info.type_.type_mask & _ZEND_TYPE_INTERSECTION_BIT, 0);
        assert!(!retval_info.type_.ptr.is_null());
    }

    #[test]
    #[cfg(php83)]
    fn returns_intersection_with_allow_null_errors() {
        let result = FunctionBuilder::new("ret_nullable_intersection", noop_handler)
            .returns(
                PhpType::Intersection(vec!["Foo".to_owned(), "Bar".to_owned()]),
                false,
                true,
            )
            .build();

        assert!(
            result.is_err(),
            "nullable intersection retval must error: nullable form is the DNF (Foo&Bar)|null"
        );
    }

    #[test]
    #[cfg(php83)]
    fn returns_dnf_emits_outer_list_with_union_bit_on_retval() {
        use crate::ffi::{_ZEND_TYPE_LIST_BIT, _ZEND_TYPE_UNION_BIT};
        use crate::types::DnfTerm;

        let entry = FunctionBuilder::new("ret_dnf", noop_handler)
            .returns(
                PhpType::Dnf(vec![
                    DnfTerm::Intersection(vec!["A".to_owned(), "B".to_owned()]),
                    DnfTerm::Single("C".to_owned()),
                ]),
                false,
                false,
            )
            .build()
            .expect("DNF return should build");

        let retval_info = unsafe { &*entry.arg_info };
        assert_ne!(retval_info.type_.type_mask & _ZEND_TYPE_LIST_BIT, 0);
        assert_ne!(retval_info.type_.type_mask & _ZEND_TYPE_UNION_BIT, 0);
        assert!(!retval_info.type_.ptr.is_null());
    }

    #[test]
    #[cfg(php83)]
    fn returns_dnf_with_allow_null_propagates_nullable_bit() {
        use crate::ffi::_ZEND_TYPE_NULLABLE_BIT;
        use crate::types::DnfTerm;

        let entry = FunctionBuilder::new("ret_nullable_dnf", noop_handler)
            .returns(
                PhpType::Dnf(vec![
                    DnfTerm::Intersection(vec!["A".to_owned(), "B".to_owned()]),
                    DnfTerm::Single("C".to_owned()),
                ]),
                false,
                true,
            )
            .build()
            .expect("nullable DNF return should build");

        let retval_info = unsafe { &*entry.arg_info };
        assert_ne!(retval_info.type_.type_mask & _ZEND_TYPE_NULLABLE_BIT, 0);
    }

    #[test]
    #[cfg(php83)]
    fn returns_empty_dnf_errors() {
        let result = FunctionBuilder::new("ret_empty_dnf", noop_handler)
            .returns(PhpType::Dnf(vec![]), false, false)
            .build();
        assert!(result.is_err());
    }
}
