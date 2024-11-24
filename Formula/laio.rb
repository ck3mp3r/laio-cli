class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.11.6"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.6/laio-0.11.6-x86_64-darwin.tgz"
      sha256 "f6a51f16fa3b9660333dda1d115545071fd6845ab5339097f8279d87f12486c9"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.6/laio-0.11.6-aarch64-darwin.tgz"
      sha256 "6dd00efca4db3885936b7ace4ca04a5d84fcc3d7544755f094d8e0f32cb79d78"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.6/laio-0.11.6-x86_64-linux.tgz"
      sha256 "3f9ab00627e1ca79a3f775e50a6ccc59d5ba37f38a3dd60ef9cc0aeaa7b79af0"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.6/laio-0.11.6-aarch64-linux.tgz"
      sha256 "82094007b7dc31c056c03bbfcce5a5ed9210f1b3eae8be295bc8dd1c82de306a"
    end
  end

  def install
    bin.install "laio"
  end
end
