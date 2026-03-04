final: prev: {
  clawclawclaw-web = final.callPackage ./web/package.nix { };

  clawclawclaw = final.callPackage ./package.nix {
    rustToolchain = final.fenix.stable.withComponents [
      "cargo"
      "clippy"
      "rust-src"
      "rustc"
      "rustfmt"
    ];
  };
}
