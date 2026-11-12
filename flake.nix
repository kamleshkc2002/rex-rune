{
  description = "Rex Rune: native Rune client for existing Rex servers";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    zig = {
      url = "github:mitchellh/zig-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      zig,
      ...
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forSystems = nixpkgs.lib.genAttrs systems;
      packageFor =
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          compiler = zig.packages.${system}."0.17.1";
        in
        pkgs.callPackage ./nix/package.nix {
          zig = compiler;
        };
    in
    {
      packages = forSystems (system: {
        default = packageFor system;
        rex-rune = self.packages.${system}.default;
        # The output's name before the rename; `nix profile upgrade rex-linux` still resolves.
        rex-linux = self.packages.${system}.default;
      });
      checks = forSystems (system: {
        rex-rune = self.packages.${system}.default;
      });
      devShells = forSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          package = self.packages.${system}.default;
        in
        {
          default = pkgs.mkShell {
            inputsFrom = [ package ];
            packages = [
              pkgs.cargo
              pkgs.rustc
              pkgs.rustfmt
              pkgs.clippy
              pkgs.nixfmt
              pkgs.statix
              zig.packages.${system}."0.17.1"
            ];
            inherit (package) GHOSTTY_SOURCE_DIR GHOSTTY_ZIG_SYSTEM_DIR ZIG;
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath package.runtimeLibraries;
          };
        }
      );
      formatter = forSystems (system: nixpkgs.legacyPackages.${system}.nixfmt);
    };
}
