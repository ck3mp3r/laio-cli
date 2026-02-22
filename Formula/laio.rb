class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.16.0"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.16.0/laio-0.16.0-aarch64-darwin.tgz"
      sha256 "d39d4a16d9638a62b272ab01bebcf99c2a94b27f11401bba1261a7a30a6e3ce8"
    else
      odie "Intel Macs are no longer supported. Please use an Apple Silicon Mac."
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.16.0/laio-0.16.0-x86_64-linux.tgz"
      sha256 "7a98fa36de00c0b7fa902dd9c4ceabeb2e089a606e075a0ee9a287550876c8eb"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.16.0/laio-0.16.0-aarch64-linux.tgz"
      sha256 "d9d8bea069eebb4d0bbc5666cfd395c332f39376b3fbd2d82aa80ab3bdf4bf4d"
    end
  end

  def install
    bin.install "laio"
  end
end
