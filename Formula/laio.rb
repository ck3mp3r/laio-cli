class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.11.5"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.5/laio-0.11.5-x86_64-darwin.tgz"
      sha256 "d2854e2200cc5d06d10320f0fcbe18ee35db0bc3f65cee5f434dd961b449a550"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.5/laio-0.11.5-aarch64-darwin.tgz"
      sha256 "384a86f2e45187a164da54744b45dc4dc70316bcc4739e899b650eff99aab58e"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.5/laio-0.11.5-x86_64-linux.tgz"
      sha256 "21b20e164fe73ca835707b9af5c1245d72654b594fbb5532b16be6a4d8e34718"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.5/laio-0.11.5-aarch64-linux.tgz"
      sha256 "b1faf454b398429e78cec63730498361a3f0c0964e2d826f2d89d67f572a7c58"
    end
  end

  def install
    bin.install "laio"
  end
end
