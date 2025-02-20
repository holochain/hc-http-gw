{
  inputs = {
    holonix = {
      url = "github:holochain/holonix?ref=main-0.4";
      inputs.crane.follows = "crane";
      inputs.rust-overlay.follows = "rust-overlay";
    };

    nixpkgs.follows = "holonix/nixpkgs";

    crane.url = "github:ipetkov/crane";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "holonix/nixpkgs";
    };
  };

  outputs = inputs@{ nixpkgs, holonix, crane, rust-overlay, ... }:
    holonix.inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = builtins.attrNames holonix.devShells;

      perSystem = { inputs', config, system, pkgs, lib, ... }:
        {
          formatter = pkgs.nixpkgs-fmt;

          devShells.default = pkgs.mkShell {
            packages = [
              (with inputs'.holonix.packages; [
                holochain
                lair-keystore
                rust
              ])
              (lib.optional pkgs.stdenv.isDarwin (with pkgs; [
                libiconv
                darwin.apple_sdk.frameworks.CoreFoundation
                darwin.apple_sdk.frameworks.SystemConfiguration
                darwin.apple_sdk.frameworks.Security
              ]))
            ];

            shellHook = ''
              export PS1='\[\033[1;34m\][holonix:\w]\$\[\033[0m\] '
            '';
          };
        };
    };
}
