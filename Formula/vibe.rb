class Vibe < Formula
  desc "Automatic project time tracking CLI tool with beautiful terminal interface"
  homepage "https://github.com/own-path/vibe"
  url "https://github.com/own-path/vibe/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "7ec25e48d7e89dd01961099bcb5fdb14740c3354d881228b1b373139faad3155"
  license "MIT"
  head "https://github.com/own-path/vibe.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args

    # Install shell completions
    generate_completions_from_executable(bin/"vibe", "completions")

    # Install shell hooks
    (share/"vibe").install "shell-hooks"
    
    # Install examples and docs
    doc.install "README.md", "CHANGELOG.md", "examples"
  end

  def post_install
    puts "🎉 Vibe has been installed successfully!"
    puts ""
    puts "To get started:"
    puts "  1. Start the daemon: vibe start"
    puts "  2. Check status: vibe status"
    puts "  3. Begin tracking: vibe session start"
    puts ""
    puts "For shell integration (optional):"
    puts "  # Bash/Zsh"
    puts "  echo 'source #{share}/vibe/shell-hooks/vibe-hook.sh' >> ~/.bashrc"
    puts "  echo 'source #{share}/vibe/shell-hooks/vibe-hook.sh' >> ~/.zshrc"
    puts ""
    puts "  # Fish"
    puts "  echo 'source #{share}/vibe/shell-hooks/vibe-hook.fish' >> ~/.config/fish/config.fish"
    puts ""
    puts "  # PowerShell"
    puts "  echo '. #{share}/vibe/shell-hooks/vibe-hook.ps1' >> $PROFILE"
    puts ""
    puts "Documentation: #{doc}"
  end

  test do
    system "#{bin}/vibe", "--version"
    system "#{bin}/vibe", "status"
  end
end