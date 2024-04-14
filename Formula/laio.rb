class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.9.7"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.9.7/laio-0.9.7-x86_64-darwin.tgz"
      sha256 "20780641ebc6659be190f573145ddbaf626d3a6496037407bb818636a8b57fd8"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.9.7/laio-0.9.7-aarch64-darwin.tgz"
      sha256 "ef29a5ee0bfa63921fdae492170552fa3950f12f5162004d563294c268987772"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.9.7/laio-0.9.7-x86_64-linux.tgz"
      sha256 "169aa62b1b89eaefeab7712c099a24324001390d5259c3c300e0938267e4e189"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.9.7/laio-0.9.7-aarch64-linux.tgz"
      sha256 "a089bbeb97cbd1d7c0aa0e6dd48760f108c60b8096fe7a65050487cef7a8cfd4"
    end
  end

  def install
    bin.install "laio"
  end
end
