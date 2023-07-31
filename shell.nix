{ pkgs, pkg, ... }:

let
	withOA = pkg.overrideAttrs (oa: {
		nativeBuildInputs = (oa.nativeBuildInputs or []) ++ (with pkgs; [
			bundler bundix
		]);
	});
in
	withOA.override {}
