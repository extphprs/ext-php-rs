mod cstring_scope;

// `CStringScope` is only reachable through `builders::ini`, itself gated on
// `php82` and the `embed` feature, so the re-export is unused elsewhere.
#[allow(unused_imports)]
pub use cstring_scope::CStringScope;
