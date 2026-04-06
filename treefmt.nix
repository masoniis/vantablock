{ ... }:
{
  projectRootFile = "flake.nix";
  programs = {
    clang-format.enable = true;
    nixfmt.enable = true;
    prettier.enable = true;
    rustfmt = {
      enable = true;
      edition = "2024"; # 2024 matches the rust edition in Cargo.toml
      # includes = [ "*.ron" ];
    };
  };

  settings.formatter.wgslfmt = {
    command = "wgslfmt"; # provided by wgsl analyzer from the flake
    includes = [
      "*.wgsl"
      "*.wesl"
    ];
  };

  settings.global.excludes = [ "**/CHANGELOG.md" ];
}
