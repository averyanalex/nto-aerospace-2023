{
  description = "NTO Capybara";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    ros.url = "github:lopsided98/nix-ros-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, ros, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ros.overlays.default ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustVersion = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        buildInputs = with pkgs; [
          libv4l
          dav1d

          wayland
          alsa-lib
          udev
          libxkbcommon
          vulkan-loader
        ];
        nativeBuildInputs = with pkgs; [
          nasm
          pkg-config
          clang
          mold
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustVersion
          ] ++ buildInputs ++ nativeBuildInputs;
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
          ROSRUST_MSG_PATH = "${pkgs.rosPackages.noetic.std-msgs}/share/std_msgs:${pkgs.rosPackages.noetic.nav-msgs}/share/nav_msgs:${pkgs.rosPackages.noetic.geometry-msgs}/share/geometry_msgs";
        };
      }
    );
}
