class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.13.0"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.13.0/laio-0.13.0-x86_64-darwin.tgz"
      sha256 "ea5594d056fcfd16d241c33e12341c5c2f01a53278ec6eca9e35e305e3f4ddc0"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.13.0/laio-0.13.0-aarch64-darwin.tgz"
      sha256 "c16028756e25254cba4234b1e179b853f2efc508346f6bffe1159f34f541fc7b"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.13.0/laio-0.13.0-x86_64-linux.tgz"
      sha256 "e91876591489fc6cdecc697d21d40871b7e485744963ae0bf216af79f42d4050"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.13.0/laio-0.13.0-aarch64-linux.tgz"
      sha256 "a778995b5606698266f76a27c21df4e4e48dbcad0d7393d5a2bf3d35fc6edd87"
    end
  end

  def install
    bin.install "laio"
  end
end
