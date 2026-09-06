//! ABI-stable description of an `ext-php-rs` extension.
//!
//! An extension built with `ext-php-rs` exports `ext_php_rs_describe_module`,
//! returning a [`Description`]. The `cargo-php` CLI loads the extension with
//! `dlopen`, calls that function and renders PHP stubs through [`ToStub`].
//! Both sides depend on this crate and nothing else from the Zend engine, so
//! the CLI builds without PHP and the layout of every `#[repr(C)]` type below
//! is the whole contract between them.
//!
//! [`VERSION`] travels inside [`Description`]. Any change to a `#[repr(C)]`
//! type here is a breaking change of this crate.
use std::vec::Vec as StdVec;

use abi::{Option, RString, Str, Vec};

pub mod abi;
mod data_type;
mod stub;

pub use data_type::DataType;
pub use stub::ToStub;

/// Version of the description ABI, the version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// A slice of strings containing documentation comments.
pub type DocComments = &'static [&'static str];

/// Representation of the extension used to generate PHP stubs.
///
/// `version` stays the first field forever: `cargo-php` reads it before
/// trusting the rest of the layout, so it has to sit at an offset that never
/// moves.
#[repr(C)]
pub struct Description {
    /// Version of `ext-php-rs-introspection` the extension was built with.
    pub version: Str,
    /// Extension description.
    pub module: Module,
}

impl Description {
    /// Creates a new description.
    ///
    /// # Parameters
    ///
    /// * `module` - The extension module representation.
    #[must_use]
    pub fn new(module: Module) -> Self {
        Self {
            version: VERSION.into(),
            module,
        }
    }
}

/// Represents a set of comments on an export.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct DocBlock(pub Vec<Str>);

impl From<&'static [&'static str]> for DocBlock {
    fn from(val: &'static [&'static str]) -> Self {
        Self(
            val.iter()
                .map(|s| (*s).into())
                .collect::<StdVec<_>>()
                .into(),
        )
    }
}

/// Represents an extension containing a set of exports.
#[repr(C)]
pub struct Module {
    /// Name of the extension.
    pub name: RString,
    /// Functions exported by the extension.
    pub functions: Vec<Function>,
    /// Classes exported by the extension.
    pub classes: Vec<Class>,
    /// Enums exported by the extension.
    pub enums: Vec<Enum>,
    /// Constants exported by the extension.
    pub constants: Vec<Constant>,
}

/// Represents an exported function.
#[repr(C)]
pub struct Function {
    /// Name of the function.
    pub name: RString,
    /// Documentation comments for the function.
    pub docs: DocBlock,
    /// Return value of the function.
    pub ret: Option<Retval>,
    /// Parameters of the function.
    pub params: Vec<Parameter>,
}

/// Represents a parameter attached to an exported function or method.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct Parameter {
    /// Name of the parameter.
    pub name: RString,
    /// Type of the parameter.
    pub ty: Option<DataType>,
    /// Whether the parameter is nullable.
    pub nullable: bool,
    /// Whether the parameter is variadic.
    pub variadic: bool,
    /// Default value of the parameter.
    pub default: Option<RString>,
}

/// Represents an exported class or interface.
#[repr(C)]
pub struct Class {
    /// Name of the class.
    pub name: RString,
    /// Documentation comments for the class.
    pub docs: DocBlock,
    /// Name of the class the exported class extends. (Not implemented #326)
    pub extends: Option<RString>,
    /// Names of the interfaces the exported class implements. (Not implemented
    /// #326)
    pub implements: Vec<RString>,
    /// Properties of the class.
    pub properties: Vec<Property>,
    /// Methods of the class.
    pub methods: Vec<Method>,
    /// Constants of the class.
    pub constants: Vec<Constant>,
    /// Whether the export is an interface rather than a class.
    pub is_interface: bool,
}

impl Class {
    /// Creates the class representing a Rust closure, exported as
    /// `RustClosure` when the `closure` feature of `ext-php-rs` is enabled.
    #[must_use]
    pub fn closure() -> Self {
        Self {
            name: "RustClosure".into(),
            docs: DocBlock(StdVec::new().into()),
            extends: Option::None,
            implements: StdVec::new().into(),
            properties: StdVec::new().into(),
            methods: vec![Method {
                name: "__invoke".into(),
                docs: DocBlock(StdVec::new().into()),
                ty: MethodType::Member,
                params: vec![Parameter {
                    name: "args".into(),
                    ty: Option::Some(DataType::Mixed),
                    nullable: false,
                    variadic: true,
                    default: Option::None,
                }]
                .into(),
                retval: Option::Some(Retval {
                    ty: DataType::Mixed,
                    nullable: false,
                }),
                r#static: false,
                visibility: Visibility::Public,
                r#abstract: false,
            }]
            .into(),
            constants: StdVec::new().into(),
            is_interface: false,
        }
    }
}

/// Represents an exported enum.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct Enum {
    /// Name of the enum.
    pub name: RString,
    /// Documentation comments for the enum.
    pub docs: DocBlock,
    /// Cases of the enum.
    pub cases: Vec<EnumCase>,
    /// Backing type of the enum.
    pub backing_type: Option<RString>,
}

/// Represents a case in an exported enum.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct EnumCase {
    /// Name of the enum case.
    pub name: RString,
    /// Documentation comments for the enum case.
    pub docs: DocBlock,
    /// Value of the enum case.
    pub value: Option<RString>,
}

/// Represents a property attached to an exported class.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct Property {
    /// Name of the property.
    pub name: RString,
    /// Documentation comments for the property.
    pub docs: DocBlock,
    /// Type of the property.
    pub ty: Option<DataType>,
    /// Visibility of the property.
    pub vis: Visibility,
    /// Whether the property is static.
    pub static_: bool,
    /// Whether the property is nullable.
    pub nullable: bool,
    /// Whether the property is readonly.
    pub readonly: bool,
    /// Default value of the property as a PHP stub string.
    pub default: Option<RString>,
}

/// Represents a method attached to an exported class.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct Method {
    /// Name of the method.
    pub name: RString,
    /// Documentation comments for the method.
    pub docs: DocBlock,
    /// Type of the method.
    pub ty: MethodType,
    /// Parameters of the method.
    pub params: Vec<Parameter>,
    /// Return value of the method.
    pub retval: Option<Retval>,
    /// Whether the method is static.
    pub r#static: bool,
    /// Visibility of the method.
    pub visibility: Visibility,
    /// Not describe method body, if is abstract.
    pub r#abstract: bool,
}

/// Represents a value returned from a function or method.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct Retval {
    /// Type of the return value.
    pub ty: DataType,
    /// Whether the return value is nullable.
    pub nullable: bool,
}

/// Enumerator used to differentiate between methods.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MethodType {
    /// A member method.
    Member,
    /// A static method.
    Static,
    /// A constructor.
    Constructor,
}

/// Enumerator used to differentiate between different method and property
/// visibilties.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Visibility {
    /// Private visibility.
    Private,
    /// Protected visibility.
    Protected,
    /// Public visibility.
    Public,
}

/// Represents an exported constant, stand alone or attached to a class.
#[repr(C)]
pub struct Constant {
    /// Name of the constant.
    pub name: RString,
    /// Documentation comments for the constant.
    pub docs: DocBlock,
    /// Value of the constant.
    pub value: Option<RString>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn description_carries_the_introspection_version() {
        let module = Module {
            name: "test".into(),
            functions: vec![].into(),
            classes: vec![].into(),
            enums: vec![].into(),
            constants: vec![].into(),
        };

        let description = Description::new(module);
        assert_eq!(description.version.str(), VERSION);
        assert_eq!(description.module.name, "test".into());
    }

    #[test]
    fn doc_block_from_slice() {
        let docs: &'static [&'static str] = &["doc1", "doc2"];
        let docs: DocBlock = docs.into();
        assert_eq!(docs.0.len(), 2);
        assert_eq!(docs.0[0], "doc1".into());
        assert_eq!(docs.0[1], "doc2".into());
    }

    #[test]
    fn closure_class_is_not_an_interface() {
        let class = Class::closure();
        assert!(!class.is_interface);
        assert_eq!(class.methods.len(), 1);
    }
}
