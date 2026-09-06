//! This could be a `.cargo/config.toml` file, however, when working in a
//! workspace only the top level config file is read. For development it's
//! easier to make this a build script, even though it does add to the compile
//! time.

fn main() {
    // `main.rs` defines a null stub for every Zend symbol listed in
    // `allowed_bindings.rs`. An extension loaded with `dlopen` resolves its
    // `zend_*` data relocations eagerly, so the stubs must be visible in the
    // binary's dynamic symbol table.
    println!("cargo:rustc-link-arg-bins=-rdynamic");
}
