class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.14.1"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.14.1/laio-0.14.1-x86_64-darwin.tgz"
      sha256 "56b0e0e99ca93d50111765370d003c9d41a1e23318b76e21a88808a53a37984d"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.14.1/laio-0.14.1-aarch64-darwin.tgz"
      sha256 "3842fa048f21ca23eaaf48cabc232af0e1a8e375803c6533714494be3f279bbf"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.14.1/laio-0.14.1-x86_64-linux.tgz"
      sha256 "8d09843e690aa5497307cb5147441d960669cd8803d2929c7a4972297fd0c942"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.14.1/laio-0.14.1-aarch64-linux.tgz"
      sha256 "258eaafd2878d8104f02494929064f1859de9539e88665240f56851a82b8d279"
    end
  end

  def install
    bin.install "laio"
  end
end
