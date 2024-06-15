class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.10.3"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.3/laio-0.10.3-x86_64-darwin.tgz"
      sha256 "df09604cd7ae5f8826ae8f830d4a6b5bf3d031d1bbddba5a60321db72320d4eb"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.3/laio-0.10.3-aarch64-darwin.tgz"
      sha256 "7af408d68a7873dc69a1775a1f09bc1697395c44e6b7710edc4ff363de9b21b1"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.3/laio-0.10.3-x86_64-linux.tgz"
      sha256 "1c8a38f8df24066c4968d409914b305ce88dcd42a7c9b9f143302f144e6456ec"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.3/laio-0.10.3-aarch64-linux.tgz"
      sha256 "1b819bf909f7c094d10fbe062f6989ed3734d5ea721e98cc1dba5ac812b482b9"
    end
  end

  def install
    bin.install "laio"
  end
end
