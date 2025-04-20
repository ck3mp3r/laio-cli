class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.14.0"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.14.0/laio-0.14.0-x86_64-darwin.tgz"
      sha256 "4abe554471957df1c08499691fd6742e29d4909c6d6a7216bd81ab94bea563c7"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.14.0/laio-0.14.0-aarch64-darwin.tgz"
      sha256 "06c536a116d97a3b4a622f7f545650982b811a0bc44318d15ca5e7fe4f9fcacd"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.14.0/laio-0.14.0-x86_64-linux.tgz"
      sha256 "206aa4d6c07707074d57bfc20782036cb2aa2878ea40646b65c84cdc5f574952"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.14.0/laio-0.14.0-aarch64-linux.tgz"
      sha256 "4699b027123f4dabad121027cf63dc3f52aca0a4891623d15f684ed7c87bea84"
    end
  end

  def install
    bin.install "laio"
  end
end
