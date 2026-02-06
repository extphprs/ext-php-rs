//! This could be a `.cargo/config.toml` file, however, when working in a
//! workspace only the top level config file is read. For development it's
//! easier to make this a build script, even though it does add to the compile
//! time.

fn main() {
    println!("cargo:rustc-link-arg-bins=-rdynamic");

    // ext-php-rs wrapper.c includes functions that call Zend engine symbols
    // only available inside a running PHP process. cargo-php never calls
    // these functions, but the linker still sees the references. Allow them
    // to remain unresolved.
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-arg-bins=-Wl,--unresolved-symbols=ignore-in-object-files");
}
