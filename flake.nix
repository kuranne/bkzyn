{
  description = "Cross-platform Nix environment for bkzyn";

  inputs = {
    # Unstable nixpkgs for the latest CLI tools
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    
    # nix-darwin for macOS system management
    darwin = {
      url = "github:lnl7/nix-darwin/master";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # home-manager for cross-platform package management
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs@{ self, nixpkgs, darwin, home-manager }:
    let
      # Packages that should be installed natively via Nix on ALL operating systems.
      # This replaces the bulk of your Brewfile and is highly cached and reproducible.
      sharedPackages = pkgs: with pkgs; [
        git git-lfs mise starship tmux zoxide zsh
        zsh-autocomplete zsh-autopair zsh-autosuggestions
        zsh-completions zsh-syntax-highlighting
        atuin bat btop eza fd fzf ripgrep tealdeer
        exiftool mpv yt-dlp imagemagick zstd
        gnupg pass neovim
      ];
    in
    {
      # 1. macOS Configuration (using nix-darwin)
      # This handles your Mac environment, mixing Nix and Homebrew beautifully.
      darwinConfigurations."macbook" = darwin.lib.darwinSystem {
        system = "aarch64-darwin"; # Assumes Apple Silicon
        modules = [
          home-manager.darwinModules.home-manager
          ({ pkgs, ... }: {
            # Tell nix-darwin to manage Homebrew for us!
            homebrew = {
              enable = true;
              
              # Fallback to Homebrew for macOS-specific tools, virtualization, 
              # or formulae not readily available in Nixpkgs.
              brews = [
                "colima"
                "molten-vk"
                "pinentry-mac"
                "ollama"
                "herdr" # Custom tap/formula
              ];
              
              # You can also declare GUI Casks here in the future!
              casks = [
                # "google-chrome"
                # "spotify"
              ];
            };

            # Inject the shared Nix packages using Home Manager
            home-manager.useGlobalPkgs = true;
            home-manager.useUserPackages = true;
            home-manager.users."user" = { # Replace "user" with your actual macOS username
              home.stateVersion = "24.05";
              home.packages = sharedPackages pkgs;
              
              # NOTE: We intentionally leave out dotfile management here, 
              # allowing your `bkzyn` CLI tool to continue handling the symlinks!
            };
          })
        ];
      };

      # 2. Linux Configuration (Standalone Home Manager)
      # Run this on Linux to get the exact same CLI environment natively!
      homeConfigurations."linux" = home-manager.lib.homeManagerConfiguration {
        pkgs = nixpkgs.legacyPackages."x86_64-linux";
        modules = [
          ({ pkgs, ... }: {
            home.username = "user";
            home.homeDirectory = "/home/user";
            home.stateVersion = "24.05";
            home.packages = sharedPackages pkgs;
          })
        ];
      };
    };
}
