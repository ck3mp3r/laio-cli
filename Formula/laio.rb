class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.10.2"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.2/laio-0.10.2-x86_64-darwin.tgz"
      sha256 "44f964f7ebcd10273ba4adbfb231dc911611898130c6a1b5d382ad4463f4ff33"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.2/laio-0.10.2-aarch64-darwin.tgz"
      sha256 "723a9cd701cc7e97347b244a1378e22a86c767e64258f83df88e0d0d16a1f89a"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.2/laio-0.10.2-x86_64-linux.tgz"
      sha256 "25c28f00f261a8b7226aa670859b4abd77828b993d1f1cc2c1a1732bb372f967"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.2/laio-0.10.2-aarch64-linux.tgz"
      sha256 "3d3a40860f20e8754aa59b3d9bd4a218da6028ac246936157777957462853dad"
    end
  end

  def install
    bin.install "laio"
  end
end
