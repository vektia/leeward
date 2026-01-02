{ pkgs }:

{
  cargoVersion = path:
    let
      cargoToml = builtins.fromTOML (builtins.readFile path);
    in
    cargoToml.workspace.package.version or cargoToml.package.version;

  buildDeps = with pkgs; [
    pkg-config
    libseccomp
  ];
}