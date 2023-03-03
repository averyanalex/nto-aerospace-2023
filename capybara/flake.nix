{
  description = "NTO Capybara";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustVersion = pkgs.rust-bin.stable.latest.default;

        buildInputs = with pkgs; [ v4l-utils ];
        nativeBuildInputs = with pkgs; [ libv4l ];
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustVersion
          ] ++ buildInputs ++ nativeBuildInputs;
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath nativeBuildInputs;
        };
      }
    );
}
