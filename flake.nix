{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/release-25.11";
    nixpkgs-unstable.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    treefmt-nix.url = "github:numtide/treefmt-nix";
    treefmt-nix.inputs.nixpkgs.follows = "nixpkgs";

    systems.url = "github:nix-systems/default";
    utils.url = "github:numtide/flake-utils";
  };
  outputs =
    {
      self,
      nixpkgs,
      nixpkgs-unstable,
      utils,
      treefmt-nix,
      systems,
    }:
    utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        pkgs-unstable = nixpkgs-unstable.legacyPackages.${system};
      in
      {
        # INFO: ------------------------
        #         local devshell
        # ------------------------------

        devShell =
          with pkgs;
          mkShell (
            {
              buildInputs = [
                # rust stuff
                pkgs-unstable.tracy-glfw
                pkgs-unstable.rust-analyzer
                pkgs-unstable.wgsl-analyzer
                rustup

                # utils
                just
                ripgrep # for justfile
                gnuplot # for benchmarks
              ]
              ++ (lib.optionals stdenv.isLinux [
                libGL
                libxkbcommon
                wayland
                pkg-config
                mesa
              ]);

              shellHook = ''
                export PATH="$HOME/.cargo/bin:$PATH"
                rustup show active-toolchain
              '';

            }
            // (lib.optionalAttrs stdenv.isLinux {
              LD_LIBRARY_PATH = lib.makeLibraryPath [
                libGL
                libxkbcommon
                wayland
              ];
            })
          );

        # INFO: -------------------------
        #         CI package sets
        # -------------------------------

        packages.default = pkgs.buildEnv {
          name = "gh action empty default profile";
          paths = [ ];
        };

        packages.formatting = pkgs.buildEnv {
          name = "gh action empty default profile";
          paths = [ pkgs-unstable.wgsl-analyzer ];
        };
      }
    )
    // (
      let
        # iterate each system and evaluate
        eachSystem = f: nixpkgs.lib.genAttrs (import systems) (system: f nixpkgs.legacyPackages.${system});
        treefmtEval = eachSystem (pkgs: treefmt-nix.lib.evalModule pkgs ./treefmt.nix);
      in
      {
        # for `nix fmt`
        formatter = eachSystem (pkgs: treefmtEval.${pkgs.system}.config.build.wrapper);
        # for `nix flake check`
        checks = eachSystem (pkgs: {
          formatting = treefmtEval.${pkgs.system}.config.build.check self;
        });
      }
    );
}
