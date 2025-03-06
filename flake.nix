{
  description = "Flake for Holochain HTTP Gateway";

  inputs = {
    holonix.url = "github:holochain/holonix?ref=main-0.4";

    nixpkgs.follows = "holonix/nixpkgs";
    flake-parts.follows = "holonix/flake-parts";
    rust-overlay.follows = "holonix/rust-overlay";
  };

  outputs = inputs@{ flake-parts, ... }: flake-parts.lib.mkFlake { inherit inputs; } {
    systems = builtins.attrNames inputs.holonix.devShells;
    perSystem = { inputs', pkgs, system, ... }: {
      _module.args.pkgs = import inputs.nixpkgs {
        inherit system;
        overlays = [ inputs.holonix.inputs.rust-overlay.overlays.default ];
      };

      formatter = pkgs.nixpkgs-fmt;

      devShells.default =
        let
          rustFromFile = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        in
        pkgs.mkShell {
          packages = (with inputs'.holonix.packages; [
            holochain
            lair-keystore
          ]) ++ ([
            pkgs.perl
            rustFromFile
          ]);

          RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;

          shellHook = ''
            export PS1='\[\033[1;34m\][holonix:\w]\$\[\033[0m\] '
          '';
        };
    };
  };
}
