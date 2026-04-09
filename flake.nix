{
  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    crane.url        = "github:ipetkov/crane";
  };

  outputs = { nixpkgs, rust-overlay, crane, ... }:
  let
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
      overlays = [ rust-overlay.overlays.default ];
    };
  in {
    devShells.${system}.default = pkgs.mkShell {
      buildInputs = with pkgs; [
        rust-bin.stable.latest.default
        cargo
        rustfmt
        clippy
        wayland
        libxkbcommon
        pkg-config
      ];
      shellHook = ''
        export LD_LIBRARY_PATH=${pkgs.wayland}/lib:${pkgs.libxkbcommon}/lib:$LD_LIBRARY_PATH
      '';
    };
  };
}
