class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.19.0"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.19.0/laio-0.19.0-aarch64-darwin.tgz"
      sha256 "d7530d49e8cc4735f717f74111d4889175ad2e1d4a16d2cabd9386b38da631b4"
    else
      odie "Intel Macs are no longer supported. Please use an Apple Silicon Mac."
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.19.0/laio-0.19.0-x86_64-linux.tgz"
      sha256 "49f7e5230badb2496317ee1dfbec2ed805d1f0b11775aba31e55ce57187e12d5"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.19.0/laio-0.19.0-aarch64-linux.tgz"
      sha256 "1d9d5801babe0b54b8a2d5a064f469a54e9239e40aeffd323664a8004b0bd501"
    end
  end

  def install
    bin.install "laio"
  end
end
