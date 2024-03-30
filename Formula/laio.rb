class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "1.0.0"

  on_macos do
    if Hardware::CPU.intel?
      url "https://example.com/download/myapp-1.0.0-x86_64-darwin.tar.gz"
      sha256 "sha256-x86_64-darwin..."
    elsif Hardware::CPU.arm?
      url "https://example.com/download/myapp-1.0.0-arm64-darwin.tar.gz"
      sha256 "sha256-arm64-darwin..."
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://example.com/download/myapp-1.0.0-x86_64-linux.tar.gz"
      sha256 "sha256-x86_64-linux..."
    elsif Hardware::CPU.arm?
      url "https://example.com/download/myapp-1.0.0-arm64-linux.tar.gz"
      sha256 "sha256-arm64-linux..."
    end
  end

  def install
    bin.install "laio"
  end
end
