{ lib, rustPlatform }:

rustPlatform.buildRustPackage {
  pname = "waybar_nzquake";
  version = "0.1.0";

  src = ./.;

  cargoLock = { lockFile = ./Cargo.lock; };

  meta = with lib; {
    description = "Waybar earthquake widget using GeoNet NZ earthquake data";
    longDescription = ''
      A Rust-based earthquake data fetcher for Waybar that retrieves earthquake
      information from the GeoNet API (api.geonet.org.nz) and outputs Waybar-compatible
      JSON format. Provides comprehensive earthquake data for New Zealand with type-safe
      domain modeling and zero-cost abstractions.
    '';
    homepage = "https://github.com/pukeko37/waybar_nzquake";
    license = licenses.mit;
    maintainers = [ ];
    platforms = platforms.linux;
  };
}
