class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.11.1"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.1/laio-0.11.1-x86_64-darwin.tgz"
      sha256 "d9f03d7feb5f35279584166ffdb274363eeb1e4a89ce20227bfcf8d7e4390f3e"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.1/laio-0.11.1-aarch64-darwin.tgz"
      sha256 "f75f0da983ac389bb0e803e5c986a3183fa0aeab29d11af471d32787015a59cd"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.1/laio-0.11.1-x86_64-linux.tgz"
      sha256 "78b2ef79a50cbea050d66f693f0ea8ae07f86f632d9e4bc21996cc9b6cce0011"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.1/laio-0.11.1-aarch64-linux.tgz"
      sha256 "dc4adeba255780d2d104f600795b4f8369e8a526aac7927b416e72e2a5b06ffd"
    end
  end

  def install
    bin.install "laio"
  end
end
