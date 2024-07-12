class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.10.4"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.4/laio-0.10.4-x86_64-darwin.tgz"
      sha256 "49b9354afad030aa52b30502779c21a8c2e7280c504a550fe792c473019976cc"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.4/laio-0.10.4-aarch64-darwin.tgz"
      sha256 "262e616bf29ddc5f5dc817ed74de22d7a7fdbe3721e939c945176e39dde1efa0"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.4/laio-0.10.4-x86_64-linux.tgz"
      sha256 "6ecdb8798b4c3549499a3a3037775c895a794ee37b6ef68e64497fe0ba0564af"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.4/laio-0.10.4-aarch64-linux.tgz"
      sha256 "b4661c9d677ce27a6e4a50bd27e3d3edf17a6739c258d59ef06e40211818b4ce"
    end
  end

  def install
    bin.install "laio"
  end
end
