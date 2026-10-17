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

        # MCP server that lets coding agents drive the running game over the bevy remote
        # protocol. Version must match the bevy release (0.19.x <-> bevy 0.18).
        bevy-brp-mcp = pkgs-unstable.rustPlatform.buildRustPackage rec {
          pname = "bevy_brp_mcp";
          version = "0.19.0";
          src = pkgs.fetchCrate {
            inherit pname version;
            hash = "sha256-Ib8UeClQwZsX6bWc6bVIfUHlKrCZqKleRzr9IwjwzOo=";
          };
          cargoHash = "sha256-PWi3zEDkkLK0P4aYcLi6MhRjCKEASRHOAvHTgPm76kc=";
          nativeBuildInputs = with pkgs; [
            cmake # aws-lc-sys
            perl
          ];
          doCheck = false;
        };
      in
      {
        # INFO: ------------------------
        #         local devshell
        # ------------------------------

        devShell =
          with pkgs;
          mkShell {
            buildInputs = [
              # rust stuff
              tracy-glfw
              pkgs-unstable.rust-analyzer
              pkgs-unstable.wgsl-analyzer
              pkgs-unstable.cargo-nextest
              rustup

              # agent tooling
              bevy-brp-mcp

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
              udev
              vulkan-loader
            ]);

            shellHook = ''
              export PATH="$HOME/.cargo/bin:$PATH"

              ${lib.optionalString stdenv.isLinux ''
                export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${
                  lib.makeLibraryPath [
                    libGL
                    libxkbcommon
                    wayland
                    vulkan-loader
                  ]
                }"
              ''}

              rustup show active-toolchain
            '';
          };

        # INFO: -------------------------
        #         CI package sets
        # -------------------------------

        packages.default = pkgs.buildEnv {
          name = "gh action empty default profile";
          paths = [ ];
        };

        packages.bevy-brp-mcp = bevy-brp-mcp;

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
        treefmtEval = eachSystem (
          pkgs:
          treefmt-nix.lib.evalModule pkgs {
            imports = [ ./treefmt.nix ];
            _module.args = {
              pkgs-unstable = nixpkgs-unstable.legacyPackages.${pkgs.stdenv.hostPlatform.system};
            };
          }
        );
      in
      {
        # for `nix fmt`
        formatter = eachSystem (pkgs: treefmtEval.${pkgs.stdenv.hostPlatform.system}.config.build.wrapper);
        # for `nix flake check`
        checks = eachSystem (pkgs: {
          formatting = treefmtEval.${pkgs.stdenv.hostPlatform.system}.config.build.check self;
        });
      }
    );
}
