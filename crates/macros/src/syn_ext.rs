use std::mem;

/// Implemented on syn types which can contain lifetimes.
pub trait DropLifetimes {
    /// Drops any lifetimes inside `self`.
    fn drop_lifetimes(&mut self);
}

impl DropLifetimes for syn::Type {
    fn drop_lifetimes(&mut self) {
        match self {
            syn::Type::Array(ty) => ty.drop_lifetimes(),
            syn::Type::FnPtr(ty) => ty.drop_lifetimes(),
            syn::Type::Group(ty) => ty.drop_lifetimes(),
            syn::Type::ImplTrait(ty) => ty.drop_lifetimes(),
            syn::Type::Paren(ty) => ty.drop_lifetimes(),
            syn::Type::Path(ty) => ty.drop_lifetimes(),
            syn::Type::Ptr(ty) => ty.drop_lifetimes(),
            syn::Type::Reference(ty) => ty.drop_lifetimes(),
            syn::Type::Slice(ty) => ty.drop_lifetimes(),
            syn::Type::TraitObject(ty) => ty.drop_lifetimes(),
            syn::Type::Tuple(ty) => ty.drop_lifetimes(),
            _ => {}
        }
    }
}

impl DropLifetimes for syn::TypeArray {
    fn drop_lifetimes(&mut self) {
        self.elem.drop_lifetimes();
    }
}

impl DropLifetimes for syn::TypeFnPtr {
    fn drop_lifetimes(&mut self) {
        self.lifetimes = None;
        self.inputs
            .iter_mut()
            .for_each(DropLifetimes::drop_lifetimes);
        self.output.drop_lifetimes();
    }
}

impl DropLifetimes for syn::NamedArg {
    fn drop_lifetimes(&mut self) {
        self.ty.drop_lifetimes();
    }
}

impl DropLifetimes for syn::ReturnType {
    fn drop_lifetimes(&mut self) {
        if let syn::ReturnType::Type(_, t) = self {
            t.drop_lifetimes();
        }
    }
}

impl DropLifetimes for syn::TypeGroup {
    fn drop_lifetimes(&mut self) {
        self.elem.drop_lifetimes();
    }
}

impl DropLifetimes for syn::TypeImplTrait {
    fn drop_lifetimes(&mut self) {
        self.bounds.drop_lifetimes();
    }
}

impl<T: Default + Clone> DropLifetimes for syn::punctuated::Punctuated<syn::TypeParamBound, T> {
    fn drop_lifetimes(&mut self) {
        *self = mem::take(self)
            .into_iter()
            .filter_map(|mut i| match &mut i {
                syn::TypeParamBound::Trait(t) => {
                    t.drop_lifetimes();
                    Some(i)
                }
                _ => None,
            })
            .collect();
    }
}

impl DropLifetimes for syn::TraitBound {
    fn drop_lifetimes(&mut self) {
        self.lifetimes = None;
        self.path.drop_lifetimes();
    }
}

impl DropLifetimes for syn::Path {
    fn drop_lifetimes(&mut self) {
        self.segments
            .iter_mut()
            .for_each(DropLifetimes::drop_lifetimes);
    }
}

impl DropLifetimes for syn::PathSegment {
    fn drop_lifetimes(&mut self) {
        if let syn::PathArguments::AngleBracketed(args) = &mut self.arguments {
            args.args = mem::take(&mut args.args)
                .into_iter()
                .filter_map(|mut i| {
                    match &mut i {
                        syn::GenericArgument::Type(t) => t.drop_lifetimes(),
                        syn::GenericArgument::AssocType(t) => t.drop_lifetimes(),
                        syn::GenericArgument::Constraint(t) => t.drop_lifetimes(),
                        syn::GenericArgument::Const(_) | syn::GenericArgument::AssocConst(_) => {}
                        _ => return None,
                    }
                    Some(i)
                })
                .collect();
        }
    }
}

impl DropLifetimes for syn::AssocType {
    fn drop_lifetimes(&mut self) {
        self.ty.drop_lifetimes();
    }
}

impl DropLifetimes for syn::Constraint {
    fn drop_lifetimes(&mut self) {
        self.bounds.drop_lifetimes();
    }
}

impl DropLifetimes for syn::TypeParen {
    fn drop_lifetimes(&mut self) {
        self.elem.drop_lifetimes();
    }
}

impl DropLifetimes for syn::TypePath {
    fn drop_lifetimes(&mut self) {
        if let Some(qself) = &mut self.qself {
            qself.ty.drop_lifetimes();
        }
        self.path.segments.drop_lifetimes();
    }
}

impl DropLifetimes for syn::TypePtr {
    fn drop_lifetimes(&mut self) {
        self.elem.drop_lifetimes();
    }
}

impl DropLifetimes for syn::TypeReference {
    fn drop_lifetimes(&mut self) {
        self.lifetime = None;
        self.elem.drop_lifetimes();
    }
}

impl DropLifetimes for syn::TypeSlice {
    fn drop_lifetimes(&mut self) {
        self.elem.drop_lifetimes();
    }
}

impl DropLifetimes for syn::TypeTraitObject {
    fn drop_lifetimes(&mut self) {
        self.bounds.drop_lifetimes();
    }
}

impl DropLifetimes for syn::TypeTuple {
    fn drop_lifetimes(&mut self) {
        self.elems
            .iter_mut()
            .for_each(DropLifetimes::drop_lifetimes);
    }
}

impl<T: DropLifetimes, P> DropLifetimes for syn::punctuated::Punctuated<T, P> {
    fn drop_lifetimes(&mut self) {
        for item in self {
            item.drop_lifetimes();
        }
    }
}
