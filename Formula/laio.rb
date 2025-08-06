class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.14.2"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.14.2/laio-0.14.2-x86_64-darwin.tgz"
      sha256 "da4b17be3076cda6e9eaa2a49abaac34b6f0969df8f7751f14b3a5ac2739f331"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.14.2/laio-0.14.2-aarch64-darwin.tgz"
      sha256 "028796807e65b3a0b2062cfaffb392beea0ed411cf068b307a8807dfb38c5cce"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.14.2/laio-0.14.2-x86_64-linux.tgz"
      sha256 "1a9fd6c5976d39520f1ad2968225b18d63e2d4ffd525dc2c676e5641198f5b3b"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.14.2/laio-0.14.2-aarch64-linux.tgz"
      sha256 "8ead0adee252e178476c610b138de8b7727843d837f0e6fe2330e7d0a4f8c06f"
    end
  end

  def install
    bin.install "laio"
  end
end
