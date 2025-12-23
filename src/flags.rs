//! Flags and enums used in PHP and the Zend engine.

use bitflags::bitflags;

#[cfg(php81)]
use crate::ffi::ZEND_ACC_ENUM;
#[cfg(not(php82))]
use crate::ffi::ZEND_ACC_REUSE_GET_ITERATOR;
use crate::ffi::{
    _IS_BOOL, CONST_CS, CONST_DEPRECATED, CONST_NO_FILE_CACHE, CONST_PERSISTENT, E_COMPILE_ERROR,
    E_COMPILE_WARNING, E_CORE_ERROR, E_CORE_WARNING, E_DEPRECATED, E_ERROR, E_NOTICE, E_PARSE,
    E_RECOVERABLE_ERROR, E_STRICT, E_USER_DEPRECATED, E_USER_ERROR, E_USER_NOTICE, E_USER_WARNING,
    E_WARNING, GC_IMMUTABLE, IS_ARRAY, IS_CALLABLE, IS_CONSTANT_AST, IS_DOUBLE, IS_FALSE,
    IS_INDIRECT, IS_ITERABLE, IS_LONG, IS_MIXED, IS_NULL, IS_OBJECT, IS_PTR, IS_REFERENCE,
    IS_RESOURCE, IS_STRING, IS_TRUE, IS_TYPE_COLLECTABLE, IS_TYPE_REFCOUNTED, IS_UNDEF, IS_VOID,
    PHP_INI_ALL, PHP_INI_PERDIR, PHP_INI_SYSTEM, PHP_INI_USER, Z_TYPE_FLAGS_SHIFT,
    ZEND_ACC_ABSTRACT, ZEND_ACC_ANON_CLASS, ZEND_ACC_CALL_VIA_TRAMPOLINE, ZEND_ACC_CHANGED,
    ZEND_ACC_CLOSURE, ZEND_ACC_CONSTANTS_UPDATED, ZEND_ACC_CTOR, ZEND_ACC_DEPRECATED,
    ZEND_ACC_DONE_PASS_TWO, ZEND_ACC_EARLY_BINDING, ZEND_ACC_FAKE_CLOSURE, ZEND_ACC_FINAL,
    ZEND_ACC_GENERATOR, ZEND_ACC_HAS_FINALLY_BLOCK, ZEND_ACC_HAS_RETURN_TYPE,
    ZEND_ACC_HAS_TYPE_HINTS, ZEND_ACC_HEAP_RT_CACHE, ZEND_ACC_IMMUTABLE,
    ZEND_ACC_IMPLICIT_ABSTRACT_CLASS, ZEND_ACC_INTERFACE, ZEND_ACC_LINKED, ZEND_ACC_NEARLY_LINKED,
    ZEND_ACC_NEVER_CACHE, ZEND_ACC_NO_DYNAMIC_PROPERTIES, ZEND_ACC_PRELOADED, ZEND_ACC_PRIVATE,
    ZEND_ACC_PROMOTED, ZEND_ACC_PROTECTED, ZEND_ACC_PUBLIC, ZEND_ACC_RESOLVED_INTERFACES,
    ZEND_ACC_RESOLVED_PARENT, ZEND_ACC_RETURN_REFERENCE, ZEND_ACC_STATIC, ZEND_ACC_STRICT_TYPES,
    ZEND_ACC_TOP_LEVEL, ZEND_ACC_TRAIT, ZEND_ACC_TRAIT_CLONE, ZEND_ACC_UNRESOLVED_VARIANCE,
    ZEND_ACC_USE_GUARDS, ZEND_ACC_USES_THIS, ZEND_ACC_VARIADIC, ZEND_EVAL_CODE,
    ZEND_HAS_STATIC_IN_METHODS, ZEND_INTERNAL_FUNCTION, ZEND_USER_FUNCTION,
};

use std::{convert::TryFrom, fmt::Display};

use crate::error::{Error, Result};

bitflags! {
    /// Flags used for setting the type of Zval.
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct ZvalTypeFlags: u32 {
        /// Undefined
        const Undef = IS_UNDEF;
        /// Null
        const Null = IS_NULL;
        /// `false`
        const False = IS_FALSE;
        /// `true`
        const True = IS_TRUE;
        /// Integer
        const Long = IS_LONG;
        /// Floating point number
        const Double = IS_DOUBLE;
        /// String
        const String = IS_STRING;
        /// Array
        const Array = IS_ARRAY;
        /// Object
        const Object = IS_OBJECT;
        /// Resource
        const Resource = IS_RESOURCE;
        /// Reference
        const Reference = IS_REFERENCE;
        /// Callable
        const Callable = IS_CALLABLE;
        /// Constant expression
        const ConstantExpression = IS_CONSTANT_AST;
        /// Void
        const Void = IS_VOID;
        /// Pointer
        const Ptr = IS_PTR;
        /// Iterable
        const Iterable = IS_ITERABLE;

        /// Interned string extended
        const InternedStringEx = Self::String.bits();
        /// String extended
        const StringEx = Self::String.bits() | Self::RefCounted.bits();
        /// Array extended
        const ArrayEx = Self::Array.bits() | Self::RefCounted.bits() | Self::Collectable.bits();
        /// Object extended
        const ObjectEx = Self::Object.bits() | Self::RefCounted.bits() | Self::Collectable.bits();
        /// Resource extended
        const ResourceEx = Self::Resource.bits() | Self::RefCounted.bits();
        /// Reference extended
        const ReferenceEx = Self::Reference.bits() | Self::RefCounted.bits();
        /// Constant ast extended
        const ConstantAstEx = Self::ConstantExpression.bits() | Self::RefCounted.bits();

        /// Reference counted
        const RefCounted = (IS_TYPE_REFCOUNTED << Z_TYPE_FLAGS_SHIFT);
        /// Collectable
        const Collectable = (IS_TYPE_COLLECTABLE << Z_TYPE_FLAGS_SHIFT);

        /// Immutable (used for the shared empty array)
        const Immutable = GC_IMMUTABLE;
    }
}

bitflags! {
    /// Flags for building classes.
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct ClassFlags: u32 {
        /// Final class or method
        const Final = ZEND_ACC_FINAL;
        /// Abstract method
        const Abstract = ZEND_ACC_ABSTRACT;
        /// Immutable `op_array` and class_entries
        /// (implemented only for lazy loading of `op_array`s)
        const Immutable = ZEND_ACC_IMMUTABLE;
        /// Function has typed arguments / class has typed props
        const HasTypeHints = ZEND_ACC_HAS_TYPE_HINTS;
        /// Top-level class or function declaration
        const TopLevel = ZEND_ACC_TOP_LEVEL;
        /// op_array or class is preloaded
        const Preloaded = ZEND_ACC_PRELOADED;

        /// Class entry is an interface
        const Interface = ZEND_ACC_INTERFACE;
        /// Class entry is a trait
        const Trait = ZEND_ACC_TRAIT;
        /// Anonymous class
        const AnonymousClass = ZEND_ACC_ANON_CLASS;
        /// Class is an Enum
        #[cfg(php81)]
        const Enum = ZEND_ACC_ENUM;
        /// Class linked with parent, interfaces and traits
        const Linked = ZEND_ACC_LINKED;
        /// Class is abstract, since it is set by any abstract method
        const ImplicitAbstractClass = ZEND_ACC_IMPLICIT_ABSTRACT_CLASS;
        /// Class has magic methods `__get`/`__set`/`__unset`/`__isset` that use guards
        const UseGuards = ZEND_ACC_USE_GUARDS;

        /// Class constants updated
        const ConstantsUpdated = ZEND_ACC_CONSTANTS_UPDATED;
        /// Objects of this class may not have dynamic properties
        const NoDynamicProperties = ZEND_ACC_NO_DYNAMIC_PROPERTIES;
        /// User class has methods with static variables
        const HasStaticInMethods = ZEND_HAS_STATIC_IN_METHODS;
        /// Children must reuse parent `get_iterator()`
        #[cfg(not(php82))]
        const ReuseGetIterator = ZEND_ACC_REUSE_GET_ITERATOR;
        /// Parent class is resolved (CE)
        const ResolvedParent = ZEND_ACC_RESOLVED_PARENT;
        /// Interfaces are resolved (CE)
        const ResolvedInterfaces = ZEND_ACC_RESOLVED_INTERFACES;
        /// Class has unresolved variance obligations
        const UnresolvedVariance = ZEND_ACC_UNRESOLVED_VARIANCE;
        /// Class is linked apart from variance obligations
        const NearlyLinked = ZEND_ACC_NEARLY_LINKED;

        /// Class cannot be serialized or unserialized
        #[cfg(php81)]
        const NotSerializable = crate::ffi::ZEND_ACC_NOT_SERIALIZABLE;
    }
}

bitflags! {
    /// Flags for building methods.
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct MethodFlags: u32 {
        /// Visibility public
        const Public = ZEND_ACC_PUBLIC;
        /// Visibility protected
        const Protected = ZEND_ACC_PROTECTED;
        /// Visibility private
        const Private = ZEND_ACC_PRIVATE;
        /// Method or property overrides private one
        const Changed = ZEND_ACC_CHANGED;
        /// Static method
        const Static = ZEND_ACC_STATIC;
        /// Final method
        const Final = ZEND_ACC_FINAL;
        /// Abstract method
        const Abstract = ZEND_ACC_ABSTRACT;
        /// Immutable `op_array` and class_entries
        /// (implemented only for lazy loading of op_arrays)
        const Immutable = ZEND_ACC_IMMUTABLE;
        /// Function has typed arguments / class has typed props
        const HasTypeHints = ZEND_ACC_HAS_TYPE_HINTS;
        /// Top-level class or function declaration
        const TopLevel = ZEND_ACC_TOP_LEVEL;
        /// `op_array` or class is preloaded
        const Preloaded = ZEND_ACC_PRELOADED;

        /// Deprecation flag
        const Deprecated = ZEND_ACC_DEPRECATED;
        /// Function returning by reference
        const ReturnReference = ZEND_ACC_RETURN_REFERENCE;
        /// Function has a return type
        const HasReturnType = ZEND_ACC_HAS_RETURN_TYPE;
        /// Function with variable number of arguments
        const Variadic = ZEND_ACC_VARIADIC;
        /// `op_array` has finally blocks (user only)
        const HasFinallyBlock = ZEND_ACC_HAS_FINALLY_BLOCK;
        /// "main" `op_array` with `ZEND_DECLARE_CLASS_DELAYED` opcodes
        const EarlyBinding = ZEND_ACC_EARLY_BINDING;
        /// Closure uses `$this`
        const UsesThis = ZEND_ACC_USES_THIS;
        /// Call through user function trampoline
        ///
        /// # Example
        /// - `__call`
        /// - `__callStatic`
        const CallViaTrampoline = ZEND_ACC_CALL_VIA_TRAMPOLINE;
        /// Disable inline caching
        const NeverCache = ZEND_ACC_NEVER_CACHE;
        /// `op_array` is a clone of trait method
        const TraitClone = ZEND_ACC_TRAIT_CLONE;
        /// Function is a constructor
        const IsConstructor = ZEND_ACC_CTOR;
        /// Function is a closure
        const Closure = ZEND_ACC_CLOSURE;
        /// Function is a fake closure
        const FakeClosure = ZEND_ACC_FAKE_CLOSURE;
        /// Function is a generator
        const Generator = ZEND_ACC_GENERATOR;
        /// Function was processed by pass two (user only)
        const DonePassTwo = ZEND_ACC_DONE_PASS_TWO;
        /// `run_time_cache` allocated on heap (user only)
        const HeapRTCache = ZEND_ACC_HEAP_RT_CACHE;
        /// `op_array` uses strict mode types
        const StrictTypes = ZEND_ACC_STRICT_TYPES;
    }
}

bitflags! {
    /// Flags for building properties.
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct PropertyFlags: u32 {
        /// Visibility public
        const Public = ZEND_ACC_PUBLIC;
        /// Visibility protected
        const Protected = ZEND_ACC_PROTECTED;
        /// Visibility private
        const Private = ZEND_ACC_PRIVATE;
        /// Property or method overrides private one
        const Changed = ZEND_ACC_CHANGED;
        /// Static property
        const Static = ZEND_ACC_STATIC;
        /// Promoted property
        const Promoted = ZEND_ACC_PROMOTED;
    }
}

bitflags! {
    /// Flags for building constants.
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct ConstantFlags: u32 {
        /// Visibility public
        const Public = ZEND_ACC_PUBLIC;
        /// Visibility protected
        const Protected = ZEND_ACC_PROTECTED;
        /// Visibility private
        const Private = ZEND_ACC_PRIVATE;
        /// Promoted constant
        const Promoted = ZEND_ACC_PROMOTED;
    }
}

bitflags! {
    /// Flags for building module global constants.
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct GlobalConstantFlags: u32 {
        /// No longer used -- always case-sensitive
        #[deprecated(note = "No longer used -- always case-sensitive")]
        const CaseSensitive = CONST_CS;
        /// Persistent
        const Persistent = CONST_PERSISTENT;
        /// Can't be saved in file cache
        const NoFileCache = CONST_NO_FILE_CACHE;
        /// Deprecated (this flag is not deprecated, it literally means the constant is deprecated)
        const Deprecated = CONST_DEPRECATED;
    }
}

bitflags! {
    /// Represents the result of a function.
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct ZendResult: i32 {
        /// Function call was successful.
        const Success = 0;
        /// Function call failed.
        const Failure = -1;
    }
}

bitflags! {
    /// Represents permissions for where a configuration setting may be set.
    pub struct IniEntryPermission: u32 {
        /// User
        const User = PHP_INI_USER;
        /// Per directory
        const PerDir = PHP_INI_PERDIR;
        /// System
        const System = PHP_INI_SYSTEM;
        /// All
        const All = PHP_INI_ALL;
    }
}

bitflags! {
    /// Represents error types when used via php_error_docref for example.
    pub struct ErrorType: u32 {
        /// Error
        const Error = E_ERROR;
        /// Warning
        const Warning = E_WARNING;
        /// Parse
        const Parse = E_PARSE;
        /// Notice
        const Notice = E_NOTICE;
        /// Core error
        const CoreError = E_CORE_ERROR;
        /// Core warning
        const CoreWarning = E_CORE_WARNING;
        /// Compile error
        const CompileError = E_COMPILE_ERROR;
        /// Compile warning
        const CompileWarning = E_COMPILE_WARNING;
        /// User error
        #[cfg_attr(php84, deprecated = "`E_USER_ERROR` is deprecated since PHP 8.4. Throw an exception instead.")]
        const UserError = E_USER_ERROR;
        /// User warning
        const UserWarning = E_USER_WARNING;
        /// User notice
        const UserNotice = E_USER_NOTICE;
        /// Strict
        const Strict = E_STRICT;
        /// Recoverable error
        const RecoverableError = E_RECOVERABLE_ERROR;
        /// Deprecated
        const Deprecated = E_DEPRECATED;
        /// User deprecated
        const UserDeprecated = E_USER_DEPRECATED;
    }
}

/// Represents the type of a function.
#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum FunctionType {
    /// Internal function
    Internal,
    /// User function
    User,
    /// Eval code
    Eval,
}

impl From<u8> for FunctionType {
    #[allow(clippy::bad_bit_mask)]
    fn from(value: u8) -> Self {
        match value.into() {
            ZEND_INTERNAL_FUNCTION => Self::Internal,
            ZEND_USER_FUNCTION => Self::User,
            ZEND_EVAL_CODE => Self::Eval,
            _ => panic!("Unknown function type: {value}"),
        }
    }
}

/// Valid data types for PHP.
#[repr(C, u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum DataType {
    /// Undefined
    Undef,
    /// `null`
    Null,
    /// `false`
    False,
    /// `true`
    True,
    /// Integer (the irony)
    Long,
    /// Floating point number
    Double,
    /// String
    String,
    /// Array
    Array,
    /// Iterable
    Iterable,
    /// Object
    Object(Option<&'static str>),
    /// Resource
    Resource,
    /// Reference
    Reference,
    /// Callable
    Callable,
    /// Constant expression
    ConstantExpression,
    /// Void
    #[default]
    Void,
    /// Mixed
    Mixed,
    /// Boolean
    Bool,
    /// Pointer
    Ptr,
    /// Indirect (internal)
    Indirect,
}

impl DataType {
    /// Returns the integer representation of the data type.
    #[must_use]
    pub const fn as_u32(&self) -> u32 {
        match self {
            DataType::Undef => IS_UNDEF,
            DataType::Null => IS_NULL,
            DataType::False => IS_FALSE,
            DataType::True => IS_TRUE,
            DataType::Long => IS_LONG,
            DataType::Double => IS_DOUBLE,
            DataType::String => IS_STRING,
            DataType::Array => IS_ARRAY,
            DataType::Object(_) => IS_OBJECT,
            DataType::Resource | DataType::Reference => IS_RESOURCE,
            DataType::Indirect => IS_INDIRECT,
            DataType::Callable => IS_CALLABLE,
            DataType::ConstantExpression => IS_CONSTANT_AST,
            DataType::Void => IS_VOID,
            DataType::Mixed => IS_MIXED,
            DataType::Bool => _IS_BOOL,
            DataType::Ptr => IS_PTR,
            DataType::Iterable => IS_ITERABLE,
        }
    }
}

// TODO: Ideally want something like this
// pub struct Type {
//     data_type: DataType,
//     is_refcounted: bool,
//     is_collectable: bool,
//     is_immutable: bool,
//     is_persistent: bool,
// }
//
// impl From<u32> for Type { ... }

impl TryFrom<ZvalTypeFlags> for DataType {
    type Error = Error;

    fn try_from(value: ZvalTypeFlags) -> Result<Self> {
        macro_rules! contains {
            ($t: ident) => {
                if value.contains(ZvalTypeFlags::$t) {
                    return Ok(DataType::$t);
                }
            };
        }

        contains!(Undef);
        contains!(Null);
        contains!(False);
        contains!(True);
        contains!(False);
        contains!(Long);
        contains!(Double);
        contains!(String);
        contains!(Array);
        contains!(Resource);
        contains!(Callable);
        contains!(ConstantExpression);
        contains!(Void);

        if value.contains(ZvalTypeFlags::Object) {
            return Ok(DataType::Object(None));
        }

        Err(Error::UnknownDatatype(0))
    }
}

impl From<u32> for DataType {
    #[allow(clippy::bad_bit_mask)]
    fn from(value: u32) -> Self {
        macro_rules! contains {
            ($c: ident, $t: ident) => {
                if (value & $c) == $c {
                    return DataType::$t;
                }
            };
        }

        contains!(IS_VOID, Void);
        contains!(IS_PTR, Ptr);
        contains!(IS_INDIRECT, Indirect);
        contains!(IS_CALLABLE, Callable);
        contains!(IS_CONSTANT_AST, ConstantExpression);
        contains!(IS_REFERENCE, Reference);
        contains!(IS_RESOURCE, Resource);
        contains!(IS_ARRAY, Array);
        contains!(IS_STRING, String);
        contains!(IS_DOUBLE, Double);
        contains!(IS_LONG, Long);
        contains!(IS_TRUE, True);
        contains!(IS_FALSE, False);
        contains!(IS_NULL, Null);

        if (value & IS_OBJECT) == IS_OBJECT {
            return DataType::Object(None);
        }

        contains!(IS_UNDEF, Undef);

        DataType::Mixed
    }
}

impl Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataType::Undef => write!(f, "Undefined"),
            DataType::Null => write!(f, "Null"),
            DataType::False => write!(f, "False"),
            DataType::True => write!(f, "True"),
            DataType::Long => write!(f, "Long"),
            DataType::Double => write!(f, "Double"),
            DataType::String => write!(f, "String"),
            DataType::Array => write!(f, "Array"),
            DataType::Object(obj) => write!(f, "{}", obj.as_deref().unwrap_or("Object")),
            DataType::Resource => write!(f, "Resource"),
            DataType::Reference => write!(f, "Reference"),
            DataType::Callable => write!(f, "Callable"),
            DataType::ConstantExpression => write!(f, "Constant Expression"),
            DataType::Void => write!(f, "Void"),
            DataType::Bool => write!(f, "Bool"),
            DataType::Mixed => write!(f, "Mixed"),
            DataType::Ptr => write!(f, "Pointer"),
            DataType::Indirect => write!(f, "Indirect"),
            DataType::Iterable => write!(f, "Iterable"),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unnecessary_fallible_conversions)]
    use super::DataType;
    use crate::ffi::{
        IS_ARRAY, IS_ARRAY_EX, IS_CONSTANT_AST, IS_CONSTANT_AST_EX, IS_DOUBLE, IS_FALSE,
        IS_INDIRECT, IS_INTERNED_STRING_EX, IS_LONG, IS_NULL, IS_OBJECT, IS_OBJECT_EX, IS_PTR,
        IS_REFERENCE, IS_REFERENCE_EX, IS_RESOURCE, IS_RESOURCE_EX, IS_STRING, IS_STRING_EX,
        IS_TRUE, IS_UNDEF, IS_VOID,
    };
    use std::convert::TryFrom;

    #[test]
    fn test_datatype() {
        macro_rules! test {
            ($c: ident, $t: ident) => {
                assert_eq!(DataType::try_from($c), Ok(DataType::$t));
            };
        }

        test!(IS_UNDEF, Undef);
        test!(IS_NULL, Null);
        test!(IS_FALSE, False);
        test!(IS_TRUE, True);
        test!(IS_LONG, Long);
        test!(IS_DOUBLE, Double);
        test!(IS_STRING, String);
        test!(IS_ARRAY, Array);
        assert_eq!(DataType::try_from(IS_OBJECT), Ok(DataType::Object(None)));
        test!(IS_RESOURCE, Resource);
        test!(IS_REFERENCE, Reference);
        test!(IS_CONSTANT_AST, ConstantExpression);
        test!(IS_INDIRECT, Indirect);
        test!(IS_VOID, Void);
        test!(IS_PTR, Ptr);

        test!(IS_INTERNED_STRING_EX, String);
        test!(IS_STRING_EX, String);
        test!(IS_ARRAY_EX, Array);
        assert_eq!(DataType::try_from(IS_OBJECT_EX), Ok(DataType::Object(None)));
        test!(IS_RESOURCE_EX, Resource);
        test!(IS_REFERENCE_EX, Reference);
        test!(IS_CONSTANT_AST_EX, ConstantExpression);
    }
}
