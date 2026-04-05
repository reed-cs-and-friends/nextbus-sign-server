{
  inputs.nixpkgs.url = "github:nixos/nixpkgs";

  outputs =
    { self, nixpkgs }:
    {
      devShells.x86_64-linux.default =
        let
          pkgs = nixpkgs.legacyPackages.x86_64-linux;
        in
        import ./shell.nix { inherit pkgs; };

      packages.x86_64-linux.default =
        let
          pkgs = nixpkgs.legacyPackages.x86_64-linux;
        in
        pkgs.rustPlatform.buildRustPackage (finalAttrs: {
          pname = "nextbus-sign-server";
          version = "0.1.0";
          src = ./.;
          cargoHash = "sha256-N5ddtZWzMydDoczA52ArDiCwJovS5Jq7fkMvm6njA7c=";
        });
    };
}
