{
  description = "ext-php-rs dev environment";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  outputs =
    { nixpkgs, rust-overlay, ... }:
    let
      system = "x86_64-linux";
      overlays = [ (import rust-overlay) ];
      pkgs = import nixpkgs { inherit system overlays; };
      php = pkgs.php.buildEnv { embedSupport = true; };
      php-dev = php.unwrapped.dev;
      php-zts = (pkgs.php.override { ztsSupport = true; }).buildEnv { embedSupport = true; };
      php-zts-dev = php-zts.unwrapped.dev;
      # ZEND_DEBUG build. Zend assertions such as the non-null handler check in
      # `zend_call_known_function` are compiled out of a release PHP, so bugs that
      # segfault in production only assert here.
      withDebug = phpPkg: phpPkg.override {
        phpAttrsOverrides = final: prev: {
          configureFlags = prev.configureFlags ++ [ "--enable-debug" ];
        };
      };
      php-debug = (withDebug pkgs.php).buildEnv { embedSupport = true; };
      php-debug-dev = php-debug.unwrapped.dev;
      php-zts-debug =
        (withDebug (pkgs.php.override { ztsSupport = true; })).buildEnv { embedSupport = true; };
      php-zts-debug-dev = php-zts-debug.unwrapped.dev;
      withAsan = phpPkg: phpPkg.override {
        stdenv = pkgs.clangStdenv;
        valgrindSupport = false;
        phpAttrsOverrides = final: prev: {
          configureFlags = prev.configureFlags ++ [
            "--enable-debug"
            "--enable-address-sanitizer"
          ];
        };
      };
      php-asan = (withAsan pkgs.php).buildEnv {
        extensions = _: [ ];
        embedSupport = true;
      };
      php-asan-dev = php-asan.unwrapped.dev;
      rust-nightly = pkgs.rust-bin.nightly.latest.default.override {
        extensions = [ "rust-src" ];
      };
      # mago is not packaged in nixpkgs; pin the upstream static musl binary so
      # local dev and CI (nhedger/setup-mago) run the exact same version.
      mago = pkgs.stdenvNoCC.mkDerivation rec {
        pname = "mago";
        version = "1.45.0";
        src = pkgs.fetchurl {
          url = "https://github.com/carthage-software/mago/releases/download/${version}/mago-${version}-x86_64-unknown-linux-musl.tar.gz";
          hash = "sha256-aNsEDrmx3uGPvf9iTBN1TPdM0z58W2CZHh/jX9mxkNE=";
        };
        installPhase = ''
          runHook preInstall
          install -Dm755 mago "$out/bin/mago"
          runHook postInstall
        '';
      };
      mkShellFor = phpPkg: phpDevPkg: pkgs.mkShell {
        buildInputs = with pkgs; [
          phpPkg
          phpDevPkg
          libclang.lib
          clang
          cargo-codspeed
          mago
        ];

        nativeBuildInputs = [ pkgs.rust-bin.stable.latest.default ];

        shellHook = ''
          export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"
          export BINDGEN_EXTRA_CLANG_ARGS="-resource-dir ${pkgs.libclang.lib}/lib/clang/${pkgs.lib.versions.major (pkgs.lib.getVersion pkgs.clang)} -isystem ${pkgs.glibc.dev}/include"
        '';
      };
      asanShell = pkgs.mkShell {
        buildInputs = [
          php-asan
          php-asan-dev
          pkgs.libclang.lib
          pkgs.clang
          pkgs.llvmPackages.llvm
        ];

        nativeBuildInputs = [ rust-nightly ];

        shellHook = ''
          export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"
          export BINDGEN_EXTRA_CLANG_ARGS="-resource-dir ${pkgs.libclang.lib}/lib/clang/${pkgs.lib.versions.major (pkgs.lib.getVersion pkgs.clang)} -isystem ${pkgs.glibc.dev}/include"
          export RUSTFLAGS="-Zsanitizer=address -Clink-arg=-Wl,-undefined,dynamic_lookup"
          export USE_ZEND_ALLOC=0
          export USE_TRACKED_ALLOC=1
          export ASAN_SYMBOLIZER_PATH="${pkgs.llvmPackages.llvm}/bin/llvm-symbolizer"
          export LSAN_OPTIONS="suppressions=$PWD/.lsan-suppressions.txt"
        '';
      };
    in
    {
      devShells.${system} = {
        default = mkShellFor php php-dev;
        zts = mkShellFor php-zts php-zts-dev;
        debug = mkShellFor php-debug php-debug-dev;
        zts-debug = mkShellFor php-zts-debug php-zts-debug-dev;
        asan = asanShell;
      };
    };
}
