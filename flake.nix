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
      # mago is not packaged in nixpkgs; pin the upstream static musl binary so
      # local dev and CI (nhedger/setup-mago) run the exact same version.
      mago = pkgs.stdenvNoCC.mkDerivation rec {
        pname = "mago";
        version = "1.29.0";
        src = pkgs.fetchurl {
          url = "https://github.com/carthage-software/mago/releases/download/${version}/mago-${version}-x86_64-unknown-linux-musl.tar.gz";
          hash = "sha256-XpnRIy+pPmrcb+qt2vK0bBSLKZAXPNzxhAC0dGRr8EY=";
        };
        installPhase = ''
          runHook preInstall
          install -Dm755 mago "$out/bin/mago"
          runHook postInstall
        '';
      };
      php82 = pkgs.php82.buildEnv { embedSupport = true; };
      php82-dev = php82.unwrapped.dev;
      php83 = pkgs.php83.buildEnv { embedSupport = true; };
      php83-dev = php83.unwrapped.dev;
      mkShellFor = phpPkg: phpDevPkg: pkgs.mkShell {
        buildInputs = with pkgs; [
          phpPkg
          phpDevPkg
          libclang.lib
          clang
          valgrind
          mago
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
        php82 = mkShellFor php82 php82-dev;
        php83 = mkShellFor php83 php83-dev;
      };
    };
}
