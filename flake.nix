{
	description = "Tantiny, a Ruby wrapper for the Tantivy search engine";

	outputs = { self
		, nixpkgs
		, ...
	}: let
		forEachSystem = with nixpkgs.lib; genAttrs systems.flakeExposed;
		eachSystemEnv' = env: fn: forEachSystem (system: fn (env system));

		globalEnv = system: rec {
			inherit system;
			pkgs = nixpkgs.legacyPackages.${system};

			pkg = pkgs.callPackage ./. { };
		};

		eachSystemEnv = eachSystemEnv' globalEnv;
	in {
		packages = eachSystemEnv (env: with env; {
			tantiny = pkg;
			default = pkg;
		});

		devShells = eachSystemEnv (env: with env; rec {
			tantiny = import ./shell.nix { inherit pkgs pkg; };
			default = tantiny;
		});
	};
}
