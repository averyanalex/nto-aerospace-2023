{
  description = "NTO Capybara";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    import-cargo.url = github:edolstra/import-cargo;
    ros.url = "github:lopsided98/nix-ros-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, import-cargo, ros, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ros.overlays.default ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustVersion = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        inherit (import-cargo.builders) importCargo;

        buildInputs = with pkgs; [
          libv4l
          dav1d

          wayland
          alsa-lib
          udev
          libxkbcommon
          vulkan-loader
          
          zlib

          # bevy x11
          # xlibsWrapper
          freetype
          fontconfig
          xorg.xorgproto
          xorg.libX11
          xorg.libXt
          xorg.libXft
          xorg.libXext
          xorg.libSM
          xorg.libICE
          # /xlibsWrapper
          xorg.libXcursor
          xorg.libXrandr
          xorg.libXi
        ];
        nativeBuildInputs = with pkgs; [
          nasm
          pkg-config
          clang
          mold
        ];

        capybara = pkgs.stdenv.mkDerivation {
          name = "capybara";
          src = self;

          inherit buildInputs;

          nativeBuildInputs = [
            (importCargo { lockFile = ./Cargo.lock; inherit pkgs; }).cargoHome
          ] ++ nativeBuildInputs;

          buildPhase = ''
            cargo build --release --offline
          '';

          installPhase = ''
            install -Dm775 ./target/release/rcmaster $out/bin/rcmaster
            install -Dm775 ./target/release/rcslave $out/bin/rcslave
          '';
        };
      in
      {
        packages = {
          default = capybara;
          capybara = capybara;
        };

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
