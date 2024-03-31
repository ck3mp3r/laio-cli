class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.9.6"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.9.6/laio-0.9.6-x86_64-darwin.tgz"
      sha256 "9ba4bec8e82a60fe706950b5b8aa479a9c04bb778d546c0538fae39daf3192d1"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.9.6/laio-0.9.6-aarch64-darwin.tgz"
      sha256 "889b4cb298570b992f223e398634c1b59c245aac76c2c4eaec4707f97d9c7efb"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.9.6/laio-0.9.6-x86_64-linux.tgz"
      sha256 "264bd0e2ebb64118612813402ed098f18d0e161699496cd78eddd119b7d5ae7a"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.9.6/laio-0.9.6-aarch64-linux.tgz"
      sha256 "6dd7f1c8334a52406139fcacc782236e6b5aa343e9791e97a52b03aca3af7d2f"
    end
  end

  def install
    bin.install "laio"
  end
end
