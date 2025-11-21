{
  description = "Waybar earthquake widget - fetches NZ earthquake data from GeoNet API";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        rustToolchain = pkgs.rust-bin.stable.latest.default;
      in {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "waybar_nzquake";
          version = "0.1.0";

          src = ./.;

          cargoLock = { lockFile = ./Cargo.lock; };

          meta = with pkgs.lib; {
            description = "Waybar earthquake widget using GeoNet NZ earthquake data";
            longDescription = ''
              A Rust-based earthquake data fetcher for Waybar that retrieves earthquake
              information from the GeoNet API (api.geonet.org.nz) and outputs Waybar-compatible
              JSON format. Provides comprehensive earthquake data for New Zealand with type-safe
              domain modeling and zero-cost abstractions.
            '';
            license = licenses.mit;
            maintainers = [ ];
            platforms = platforms.linux;
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustToolchain
            pkgs.rust-analyzer
          ];

          shellHook = ''
            echo "waybar_nzquake development environment"
            echo "Rust toolchain: ${rustToolchain}"
            echo "Run 'cargo build' to build the project"
            echo "Run 'cargo run' to fetch recent NZ earthquake data"
            echo "No API key required - uses public GeoNet API"
          '';
        };

        apps.default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/waybar_nzquake";
        };
      });
}
