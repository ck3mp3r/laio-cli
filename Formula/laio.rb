class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.11.3"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.3/laio-0.11.3-x86_64-darwin.tgz"
      sha256 "d5c0bede8ac6a8b0ce13d645d5206aa30b076e87424d387b880150832d87a6a1"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.3/laio-0.11.3-aarch64-darwin.tgz"
      sha256 "fde0bc8d7807590b4970753b312c9f811dec76b7c750fcf1d7046a13346efa95"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.3/laio-0.11.3-x86_64-linux.tgz"
      sha256 "f7ee66f8150db084c206c44b7957218d334485c82efacc7a783a02dc99cfaa12"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.3/laio-0.11.3-aarch64-linux.tgz"
      sha256 "2031f325c884e5f616927f5833fa1fa42a7991a1424d76ca783f2540c35e1cfb"
    end
  end

  def install
    bin.install "laio"
  end
end
