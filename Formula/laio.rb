class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.18.0"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.18.0/laio-0.18.0-aarch64-darwin.tgz"
      sha256 "4cc86047a924989b8f2e92570033b2c94407bac05b0add1ee7541e66caad8013"
    else
      odie "Intel Macs are no longer supported. Please use an Apple Silicon Mac."
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.18.0/laio-0.18.0-x86_64-linux.tgz"
      sha256 "7618fbd17265838e638636c57b28e47e1c559067038efecb296ac1a1ae595649"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.18.0/laio-0.18.0-aarch64-linux.tgz"
      sha256 "4ba1fd272f2711c38922d6ca0c386f3b2642d3d781b504a6bc9c4392aa0e7cf8"
    end
  end

  def install
    bin.install "laio"
  end
end
