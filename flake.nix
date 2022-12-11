{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nci = {
      url = "github:yusdacra/nix-cargo-integration";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = { self, nixpkgs, nci }: nci.lib.makeOutputs {
    root = ./.;
    config = common: {
      outputs = {
        defaults = {
          app = "hyperbolic";
          package = "hyperbolic";
        };
      };
      runtimeLibs = with common.pkgs; [vulkan-loader wayland libxkbcommon];
    };
  };
}
