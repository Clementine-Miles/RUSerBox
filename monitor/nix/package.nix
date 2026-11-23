# Build the headless Monitor from the ServerBox workspace.
{ lib
, rustPlatform
, pkg-config
, sqlite
, nix-update-script
}:

let
  # The monorepo root: `monitor` is a workspace member, and the crate it
  # depends on (`crates/sbm_parser`) is a sibling, so the source cannot be
  # `monitor/` alone.
  src = lib.cleanSource ../..;

in
rustPlatform.buildRustPackage {
  pname = "server-box-monitor";
  version = "0-unstable";
  inherit src;

  cargoLock.lockFile = ../../Cargo.lock;

  buildAndTestSubdir = "monitor";

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ sqlite ];

  # sqlx's macros read `monitor/.sqlx` rather than a live database, which is
  # what makes an offline build possible at all.
  SQLX_OFFLINE = "true";

  # The tests want a database and a network. Left off rather than patched
  # around: `cargo test --workspace` on a developer's machine is where they
  # belong, and a package that pretends to run them is worse than one that
  # says it does not.
  doCheck = false;

  passthru.updateScript = nix-update-script { };

  meta = with lib; {
    description = "Server-side monitoring agent for ServerBox";
    homepage = "https://github.com/lollipopkit/flutter_server_box";
    license = licenses.agpl3Only;
    mainProgram = "server_box_monitor";
    platforms = platforms.linux;
  };
}
