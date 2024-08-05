class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.10.5"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.5/laio-0.10.5-x86_64-darwin.tgz"
      sha256 "859eb70dfc95fb797116aa6f18d2575fc4af6097354f9f5bddce128f1c4bbfb8"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.5/laio-0.10.5-aarch64-darwin.tgz"
      sha256 "b6a410764bda5e4a58f1132ad3c31b7a100dfa3dec34dcfb9f74e70294ad133f"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.5/laio-0.10.5-x86_64-linux.tgz"
      sha256 "f7a87559b5b8b7c1560f479e931be8805100741e2bf4ecfb2fc3c6430ee8f1e0"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.5/laio-0.10.5-aarch64-linux.tgz"
      sha256 "0f589f3356e4b276e179f6ec12a19239f94ce8f1525b6af613e08dedf932b643"
    end
  end

  def install
    bin.install "laio"
  end
end
