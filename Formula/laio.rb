class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.15.1"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.15.1/laio-0.15.1-aarch64-darwin.tgz"
      sha256 "78639b59cc8806e7689feb098793cc1a4732d7a8c60b6f08c21b6ffbc66a17d9"
    else
      odie "Intel Macs are no longer supported. Please use an Apple Silicon Mac."
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.15.1/laio-0.15.1-x86_64-linux.tgz"
      sha256 "74098e29e3dacc6066358752e160d11f693e850593be8e3ada7bfe0cc2b31741"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.15.1/laio-0.15.1-aarch64-linux.tgz"
      sha256 "79b1096539d7adda1f6a472880e1794394552ed606314b2900cae12410169a93"
    end
  end

  def install
    bin.install "laio"
  end
end
