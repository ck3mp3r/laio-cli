class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.14.3"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.14.3/laio-0.14.3-aarch64-darwin.tgz"
      sha256 "33643d682abd23761c84b6c3bc9ab4a1f026a401c68c6f3db1a812b74750d977"
    else
      odie "Intel Macs are no longer supported. Please use an Apple Silicon Mac."
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.14.3/laio-0.14.3-x86_64-linux.tgz"
      sha256 "b7302dd5c42250474923230771239276471d47888516e40e1320650befcb69a9"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.14.3/laio-0.14.3-aarch64-linux.tgz"
      sha256 "50706fc5a04fb8ff85c8bc0d2f9c0d21d6233a8b9a051a01a61e25b6708313d6"
    end
  end

  def install
    bin.install "laio"
  end
end
