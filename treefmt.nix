{ ... }:
{
  projectRootFile = "flake.nix";
  programs = {
    clang-format.enable = true;
    nixfmt.enable = true;
    prettier.enable = true;
  };

  settings.formatter.rustfmt = {
    command = "rustfmt";
    options = [
      "--edition"
      "2024"
    ];
    includes = [ "*.rs" ];
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
