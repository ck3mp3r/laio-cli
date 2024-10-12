class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.11.2"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.2/laio-0.11.2-x86_64-darwin.tgz"
      sha256 "4a508ee1ca521a22b67514c10d473aae978f3bd0f17b46acdee16f3b82428d3f"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.2/laio-0.11.2-aarch64-darwin.tgz"
      sha256 "64a7b78c435c3231dfe2e581b31449a12aaf48b5836440325b1499ec9771b58f"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.2/laio-0.11.2-x86_64-linux.tgz"
      sha256 "3315f47737b3939df6029c1b5808e7c347d84b3c5619800cfae694f69b4a4c32"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.2/laio-0.11.2-aarch64-linux.tgz"
      sha256 "4a8f901e661846f2425f5754c6a057fe5b2a79072a37bfc18c80ad305f3eb5ce"
    end
  end

  def install
    bin.install "laio"
  end
end
