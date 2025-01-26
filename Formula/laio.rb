class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.12.1"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.12.1/laio-0.12.1-x86_64-darwin.tgz"
      sha256 "d43ff751acb7d93b888a7310ddbda099371ebc2e3ed3a9cfb937b268481a497c"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.12.1/laio-0.12.1-aarch64-darwin.tgz"
      sha256 "bb580c466099bc0d4ef6954136e2ee04a7534206869f42b4cad07e503f07b1dc"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.12.1/laio-0.12.1-x86_64-linux.tgz"
      sha256 "c230b8dbe3b0e5aee083bc8c2336753a0b226d94e71d0cb8a1bc7ff0bc8081a7"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.12.1/laio-0.12.1-aarch64-linux.tgz"
      sha256 "9d9f0d2fb54640ac73e87c3b9fbbb58a8bd28d9281ebcf42311c221b84ac09cb"
    end
  end

  def install
    bin.install "laio"
  end
end
