{
  # `description` is just metadata — shows up in `nix flake show` / `nix flake metadata`.
  description = "NyxDB dev environment";

  # `inputs` = the *dependencies of this flake itself* (not of your Rust code).
  # Every input gets pinned by exact commit hash in flake.lock the first time
  # you run any `nix` command here. That lockfile is what makes flakes
  # reproducible — no more "works on my machine because my channel updated".
  #
  # nixpkgs is the giant repo of package definitions (rustc, cargo, gcc, ...).
  # "nixos-unstable" tracks the latest tested packages. There are also
  # versioned stable branches like "nixos-24.11" if you want things to move
  # less.
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  # `outputs` is a function. Nix calls it with your resolved inputs and expects
  # back an attribute set describing what this flake *produces*: devShells,
  # packages, apps, etc. This is the only part of a flake that's "real code".
  outputs = { self, nixpkgs }:
    let
      system = "aarch64-darwin";

      # `import nixpkgs { inherit system; }` turns the nixpkgs *source* (an input,
      # just files) into `pkgs`, an attribute set of actually-usable package
      # derivations built for your system.
      pkgs = import nixpkgs { inherit system; };
    in
    {
      # `devShells.<system>.default` is what `nix develop` looks for.
      # mkShell doesn't build a package — it builds an *environment*: a shell
      # with `buildInputs` on PATH, nothing more. Nothing here gets installed
      # globally on your machine; it only exists while the shell is active.
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = [
          pkgs.rustc # the Rust compiler
          pkgs.cargo # Rust's build tool / package manager
        ];

        # `shellHook` is shell script that runs every time you enter this
        # environment via `nix develop`. Useful for a sanity-check message.
        shellHook = ''
          echo "NyxDB dev shell ready. rustc: $(rustc --version)"
        '';
      };
    };
}
