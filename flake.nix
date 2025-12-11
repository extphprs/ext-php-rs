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
    in
    {
      devShells.${system} = {
        default = pkgs.mkShell {
          buildInputs = with pkgs; [
            php
            php-dev
            libclang.lib
            clang
            valgrind
          ];

          nativeBuildInputs = [ pkgs.rust-bin.stable.latest.default ];

          shellHook = ''
            export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"
            export GUNGRAUN_VALGRIND_INCLUDE="${pkgs.valgrind.dev}/include"
          '';
        };
      };
    };
}
