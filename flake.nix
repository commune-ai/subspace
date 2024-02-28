{
  description = "Commune's Subspace Node";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/23.11";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    naersk.url = "github:nix-community/naersk";
  };

  outputs = { nixpkgs, rust-overlay, flake-utils, naersk, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        generalBuildInputs = with pkgs; [pkg-config rocksdb zstd.dev bashInteractive];
        buildInputs = with pkgs;
          if pkgs.stdenv.isLinux
          then generalBuildInputs ++ [jemalloc]
          else generalBuildInputs;
        nativeBuildInputs = with pkgs; [git rust clang protobuf];

        naersk' = pkgs.callPackage naersk {
          cargo = rust;
          rustc = rust;
        };
      in {
        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;

          env = {
            LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
            ROCKSDB_LIB_DIR = "${pkgs.rocksdb}/lib";
            ZSTD_SYS_USE_PKG_CONFIG = "true";
          } // nixpkgs.lib.optionalAttrs pkgs.stdenv.isLinux { JEMALLOC_OVERRIDE = "${pkgs.jemalloc}/lib/libjemalloc.so"; };
        };

        packages.default = naersk'.buildPackage {
          inherit buildInputs nativeBuildInputs;
          src = ./.;
        };
      }
    );
}
