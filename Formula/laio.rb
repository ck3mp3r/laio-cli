class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.13.1"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.13.1/laio-0.13.1-x86_64-darwin.tgz"
      sha256 "ac1f40fcd7d2802f59da762e2127442ea3ae3a6ea00cafd4716c0f118fe3d484"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.13.1/laio-0.13.1-aarch64-darwin.tgz"
      sha256 "48c5eadb50b079ffddbdb28b128cb36af5103d19d47646625aba88eef6e4627f"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.13.1/laio-0.13.1-x86_64-linux.tgz"
      sha256 "1f5143ec11400ee41811d546883ae2ef71be00ac218f4d5f27bbd0ae0a1038f2"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.13.1/laio-0.13.1-aarch64-linux.tgz"
      sha256 "179899f109c9bff3bc0197f92786105279041bd8ec39b6126c380d483ca6ed7f"
    end
  end

  def install
    bin.install "laio"
  end
end
