class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.12.0"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.12.0/laio-0.12.0-x86_64-darwin.tgz"
      sha256 "9dd094eb5674313596245b0983ad8112175328aae199ded1e375af3c27dd4c0d"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.12.0/laio-0.12.0-aarch64-darwin.tgz"
      sha256 "2c1fc563844417ceb8776ae4a5dd204880e8831dd5b495cb066977e8e35ce6bf"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.12.0/laio-0.12.0-x86_64-linux.tgz"
      sha256 "ef6dacf0b0d546ba1209800d65d78bf2c1441eff85f50628273afafab3cd4fbc"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.12.0/laio-0.12.0-aarch64-linux.tgz"
      sha256 "6f3600cb82ccc2e0eb7d0403227ba5c9545be1ca5022a61ed447bb3f71c6a892"
    end
  end

  def install
    bin.install "laio"
  end
end
