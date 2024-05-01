class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.10.0"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.0/laio-0.10.0-x86_64-darwin.tgz"
      sha256 "a07fc01a35ff438b773a31614d335bf31148159fc1e636e63e13b92c9c182d3c"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.0/laio-0.10.0-aarch64-darwin.tgz"
      sha256 "ea9b2f0dec84b8bb8116353ff06bac647fb4fd5a11f34d14a69a9fd00a63b7c8"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.0/laio-0.10.0-x86_64-linux.tgz"
      sha256 "3c66c305e279ad8033eaa2d11e9b78a46ed486b054f342f8bb2e1fb81a315374"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.0/laio-0.10.0-aarch64-linux.tgz"
      sha256 "c2b50a1ad7a55db2b911856b9d5f75b6138189f70bb36013353df58e050474f8"
    end
  end

  def install
    bin.install "laio"
  end
end
