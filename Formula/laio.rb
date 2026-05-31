class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.17.0"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.17.0/laio-0.17.0-aarch64-darwin.tgz"
      sha256 "d3571a21ab293755daf38dd7d904843700192d2e80b7dc20f8516a6fb1e5c17c"
    else
      odie "Intel Macs are no longer supported. Please use an Apple Silicon Mac."
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.17.0/laio-0.17.0-x86_64-linux.tgz"
      sha256 "42316641da0275930012f91f03913caf25cf1d6a149c7b587565cfd821a1e524"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.17.0/laio-0.17.0-aarch64-linux.tgz"
      sha256 "a060834dfddb393050ef9a7fe765924367adc224163024506b33e8e6c82ad749"
    end
  end

  def install
    bin.install "laio"
  end
end
