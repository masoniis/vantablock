{ pkgs, pkgs-unstable, ... }:
let
  wgsl-analyzer = pkgs-unstable.wgsl-analyzer;
  # treefmt requires formatters not to touch files if unchanged.
  # wgslfmt always updates mtime even when unchanged, which triggers false positives in treefmt --ci.
  wgslfmt-wrapper = pkgs.writeShellScriptBin "wgslfmt-treefmt" ''
    for file in "$@"; do
      if ! ${wgsl-analyzer}/bin/wgslfmt --check "$file" >/dev/null 2>&1; then
        ${wgsl-analyzer}/bin/wgslfmt "$file"
      fi
    done
  '';
in
{
  projectRootFile = "flake.nix";
  programs = {
    clang-format.enable = true;
    nixfmt.enable = true;
    prettier.enable = true;
    rustfmt.enable = true;
    rustfmt.edition = "2024";
  };

  settings.formatter.wgslfmt = {
    command = "${wgslfmt-wrapper}/bin/wgslfmt-treefmt";
    includes = [
      "*.wgsl"
      "*.wesl"
    ];
  };

  settings.global.excludes = [ "CHANGELOG.md" ];
}
