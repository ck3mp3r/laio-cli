class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.16.5"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.16.5/laio-0.16.5-aarch64-darwin.tgz"
      sha256 "e679013f9bb9deb0e31c440d94d5a173b4b7a0729076e6c63c736608ea220e18"
    else
      odie "Intel Macs are no longer supported. Please use an Apple Silicon Mac."
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.16.5/laio-0.16.5-x86_64-linux.tgz"
      sha256 "7dfb8d0db15a59c5380dc2286e87059653d0bd50a208275de3695152401c4bb7"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.16.5/laio-0.16.5-aarch64-linux.tgz"
      sha256 "c08dad52ea7101d05f1bf5f2b608cbb767b44a523f67f8c3c2dec8fa3c8d6514"
    end
  end

  def install
    bin.install "laio"
  end
end
