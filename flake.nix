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
      mkShellFor = phpPkg: phpDevPkg: pkgs.mkShell {
        buildInputs = with pkgs; [
          phpPkg
          phpDevPkg
          libclang.lib
          clang
          valgrind
        ];

        nativeBuildInputs = [ pkgs.rust-bin.stable.latest.default ];

        shellHook = ''
          export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"
          export BINDGEN_EXTRA_CLANG_ARGS="-resource-dir ${pkgs.libclang.lib}/lib/clang/${pkgs.lib.versions.major (pkgs.lib.getVersion pkgs.clang)} -isystem ${pkgs.glibc.dev}/include"
        '';
      };
    in
    {
      devShells.${system} = {
        default = mkShellFor php php-dev;
        zts = mkShellFor php-zts php-zts-dev;
      };
    };
}
