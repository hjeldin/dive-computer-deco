{
  description = "A Nix-flake-based Rust development environment";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1.*.tar.gz";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      supportedSystems = [ "x86_64-linux" "thumbv7em-none-eabi" ];
      forEachSupportedSystem = f: nixpkgs.lib.genAttrs supportedSystems (system: 
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default self.overlays.default ];
          };
          libPath = with pkgs; lib.makeLibraryPath [
            wayland-protocols
            wayland
            libxkbcommon
            libGL
          ];
        in
        f { inherit pkgs libPath; });
    in
    {
      overlays.default = final: prev: {
        rustToolchain =
          let
            rust = prev.rust-bin;
          in
          if builtins.pathExists ./rust-toolchain.toml then
            rust.fromRustupToolchainFile ./rust-toolchain.toml
          else if builtins.pathExists ./rust-toolchain then
            rust.fromRustupToolchainFile ./rust-toolchain
          else
            rust.nightly.latest.default.override {
              extensions = [ "rust-src" "rustfmt" ];
            };
      };

      devShells = forEachSupportedSystem ({ pkgs, libPath }: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            SDL2
            rustToolchain
            openssl
            pkg-config
            cargo-deny
            cargo-edit
            cargo-watch
            rust-analyzer
            pkg-config
            llvm
            clang
            libclang
            flip-link
            probe-rs-tools
            usbutils

            linuxPackages_latest.perf

            gnuplot

            openocd

            gdb
            gef
            rustup

            # GUI and desktop application dependencies
            cmake
            ninja
            fontconfig
            freetype
            libGL
            libGLU
            wayland
            wayland-protocols
            libxkbcommon

            # Rust Embedded
            (rust-bin.stable.latest.default.override {
              extensions = [ "rust-src" ];
              targets = [ "thumbv7em-none-eabihf" "thumbv6m-none-eabi" ];
            })
          ];

          buildInputs = with pkgs; [
            SDL2
            xorg.libX11
            xorg.libXft
            xorg.libXext
            xorg.libXinerama
            xorg.libXcursor
            xorg.libXrender
            xorg.libXfixes
            fontconfig
            freetype
            libGL
            libGLU
            wayland
            wayland-protocols
            libxkbcommon
          ];

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          env = {
            # Required by rust-analyzer
            RUST_SRC_PATH = "${pkgs.rustToolchain}/lib/rustlib/src/rust/library";
            LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib";
          };

          shellHook = ''
            echo "Using Rust toolchain: $(rustc --version)"

            export CARGO_HOME="$HOME/.cargo"
            export RUSTUP_HOME="$HOME/.rustup"
            export LD_LIBRARY_PATH="${libPath}"
            echo "Set LD_LIBRARY_PATH to ${libPath}"
            mkdir -p "$CARGO_HOME" "$RUSTUP_HOME"
          '';
        };
      });
    };
}
