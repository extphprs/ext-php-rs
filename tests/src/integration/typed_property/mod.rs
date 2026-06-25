use ext_php_rs::builders::{ClassBuilder, ClassProperty};
use ext_php_rs::flags::{DataType, PropertyFlags};
use ext_php_rs::prelude::*;
use ext_php_rs::types::PhpType;

#[php_class]
#[php(modifier = inject_typed_props)]
pub struct TypedPropClass;

#[php_impl]
impl TypedPropClass {
    pub fn __construct() -> Self {
        Self
    }
}

#[php_class]
pub struct TypedPropFooClass;

#[php_impl]
impl TypedPropFooClass {
    pub fn __construct() -> Self {
        Self
    }
}

#[php_class]
pub struct TypedPropBarClass;

#[php_impl]
impl TypedPropBarClass {
    pub fn __construct() -> Self {
        Self
    }
}

fn inject_typed_props(b: ClassBuilder) -> ClassBuilder {
    let mut b = b
        .property(ClassProperty {
            name: "intProp".into(),
            flags: PropertyFlags::Public,
            default: None,
            docs: &[],
            ty: Some(PhpType::Simple(DataType::Long)),
            nullable: false,
            readonly: false,
            default_stub: None,
        })
        .property(ClassProperty {
            name: "nullableIntProp".into(),
            flags: PropertyFlags::Public,
            default: None,
            docs: &[],
            ty: Some(PhpType::Simple(DataType::Long)),
            nullable: true,
            readonly: false,
            default_stub: None,
        })
        .property(ClassProperty {
            name: "stringOrIntProp".into(),
            flags: PropertyFlags::Public,
            default: None,
            docs: &[],
            ty: Some(PhpType::Union(vec![DataType::String, DataType::Long])),
            nullable: false,
            readonly: false,
            default_stub: None,
        })
        .property(ClassProperty {
            name: "fooProp".into(),
            flags: PropertyFlags::Public,
            default: None,
            docs: &[],
            ty: Some(PhpType::Simple(DataType::Object(Some("TypedPropFooClass")))),
            nullable: false,
            readonly: false,
            default_stub: None,
        })
        .property(ClassProperty {
            name: "fooOrBarProp".into(),
            flags: PropertyFlags::Public,
            default: None,
            docs: &[],
            ty: Some(PhpType::ClassUnion(vec![
                "TypedPropFooClass".into(),
                "TypedPropBarClass".into(),
            ])),
            nullable: false,
            readonly: false,
            default_stub: None,
        });

    #[cfg(php81)]
    {
        b = b.property(ClassProperty {
            name: "intersectProp".into(),
            flags: PropertyFlags::Public,
            default: None,
            docs: &[],
            ty: Some(PhpType::Intersection(vec![
                "Countable".into(),
                "Traversable".into(),
            ])),
            nullable: false,
            readonly: false,
            default_stub: None,
        });
    }

    #[cfg(php83)]
    {
        b = b.property(ClassProperty {
            name: "dnfProp".into(),
            flags: PropertyFlags::Public,
            default: None,
            docs: &[],
            ty: Some(PhpType::Dnf(vec![
                ext_php_rs::types::DnfTerm::Intersection(vec![
                    "Countable".into(),
                    "Traversable".into(),
                ]),
                ext_php_rs::types::DnfTerm::Single("TypedPropFooClass".into()),
            ])),
            nullable: false,
            readonly: false,
            default_stub: None,
        });
    }

    b
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .class::<TypedPropFooClass>()
        .class::<TypedPropBarClass>()
        .class::<TypedPropClass>()
}

#[cfg(test)]
mod tests {
    #[test]
    fn typed_property_metadata_matches_reflection() {
        assert!(crate::integration::test::run_php(
            "typed_property/typed_property.php"
        ));
    }
}
