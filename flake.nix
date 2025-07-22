{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    nixpkgs,
    crane,
    fenix,
    ...
  }: let
    systems = ["x86_64-linux" "aarch64-darwin"];

    perSystem = f:
      nixpkgs.lib.foldAttrs nixpkgs.lib.mergeAttrs {}
      (map (s: nixpkgs.lib.mapAttrs (_: v: {${s} = v;}) (f s)) systems);
  in
    perSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [fenix.overlays.default];
      };

      craneLib =
        (crane.mkLib pkgs).overrideToolchain
        pkgs.fenix.stable.toolchain;

      src = pkgs.lib.fileset.toSource {
        root = ./.;
        fileset = pkgs.lib.fileset.unions [
          (craneLib.fileset.commonCargoSources ./.)
          ./src/snapshots
          ./tests
        ];
      };

      args = {
        inherit src;
        strictDeps = true;
      };

      cargoArtifacts = craneLib.buildDepsOnly args;
      package = craneLib.buildPackage (args // {inherit cargoArtifacts;});
      cargoClippyExtraArgs = "--all-targets -- --deny warnings";
    in {
      devShells.default = craneLib.devShell {
        packages = with pkgs; [cargo-insta rust-analyzer-nightly];
      };

      checks = {
        inherit package;

        fmt = craneLib.cargoFmt (args // {inherit cargoArtifacts;});
        fmt-toml = craneLib.taploFmt {src = pkgs.lib.sources.sourceFilesBySuffices src [".toml"];};
        test = craneLib.cargoTest (args // {inherit cargoArtifacts;});
        clippy = craneLib.cargoClippy (args // {inherit cargoArtifacts cargoClippyExtraArgs;});
      };

      packages.default = package;
      apps.default = {
        type = "app";
        program = "${package}/bin/woof";
        meta.description = "A simple translation code generator";
      };
    });
}
