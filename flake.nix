{
  description = "A platformer written in Rust using Bevy";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    {
      self,
      nixpkgs,
      ...
    }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        system = system;
        config.allowUnfree = true;
      };

      projectName = "bevy-platformer";
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          vscode-langservers-extracted
          rustup
          pkg-config
          # pkgs.cudaPackages.cudatoolkit
        ];
        buildInputs = with pkgs; [
          alsa-lib.dev

          libevdev
          udev.dev
    #       pkgs.mpv-unwrapped
    #       pkgs.sdl2-compat
	  # ffmpeg-full
        ];

        # LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
        #   pkgs.vulkan-loader
        #   pkgs.libGL
        #   pkgs.libxkbcommon
        #   pkgs.wayland
        #   pkgs.xorg.libX11
        #   pkgs.xorg.libXcursor
        #   pkgs.xorg.libXi
        #   pkgs.xorg.libXrandr
        #   pkgs.wayland
        #   pkgs.mpv-unwrapped
        #   pkgs.sdl2-compat
        #   pkgs.cudaPackages.cudatoolkit
        # ];
        # CUDA_ROOT = "${pkgs.cudaPackages.cudatoolkit}";

        shellHook = ''
          printf '\x1b[36m\x1b[1m\x1b[4mTime to develop ${projectName}!\x1b[0m\n\n'

	  LD_LIBRARY_PATH=${pkgs.stdenv.cc.cc.lib}/lib/:${pkgs.libGL}/lib/:${pkgs.glib.out}/lib:/run/opengl-driver/lib/:$LD_LIBRARY_PATH
        '';
      };
    };
}
