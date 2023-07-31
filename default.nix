{ stdenv, lib
, buildRubyGem
, bundlerEnv
, ruby
, rustPlatform, cargo, rustc }:

let
	gemName = "tantiny";
	version = "unstable";

	env = bundlerEnv {
		name = "${gemName}-${version}-env";
		inherit version;
		inherit ruby;
		gemdir = ./.;
	};

in buildRubyGem rec {
	inherit gemName version;
	pname = gemName;

	src = ./.;

	cargoDeps = rustPlatform.importCargoLock {
	  lockFile = ./Cargo.lock;
	};

	propagatedBuildInputs = [ env ];

	nativeBuildInputs = with rustPlatform; [
		cargoSetupHook cargo rustc
	];


	postUnpack = ''
		unset -f cargoSetupPostUnpackHook

		cargoSetupPostUnpackHook () {
			true
		}

		export CARGO_HOME="$(realpath ./.cargo)";
	'';

	meta = with lib; {
		description = "Ruby wrapper for the Tantivy search engine";
		homepage = "https://github.com/anna328p/tantiny";
		license = licenses.mit;
		maintainers = with maintainers; [ anna328p ];
	};
}
