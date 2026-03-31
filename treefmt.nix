{ ... }:
{
  projectRootFile = "flake.nix";
  programs = {
    clang-format.enable = true;
    nixfmt.enable = true;
    prettier.enable = true;
    rustfmt = {
      enable = true;
      edition = "2021"; # 2021 matches the rustfmt edition my nvim uses, so im matching this for now
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

  settings.global.excludes = [ "CHANGELOG.md" ];
}
