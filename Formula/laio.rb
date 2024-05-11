class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.10.1"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.1/laio-0.10.1-x86_64-darwin.tgz"
      sha256 "e1a07dbb1f8dffed96df0776f6ce383fff13d6d117e2dc0ec13297c1d7b6f3ca"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.1/laio-0.10.1-aarch64-darwin.tgz"
      sha256 "d4b490dd58d0298ae628953a226bb693770ce0dec9864ff9aae8e85d2192e79e"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.1/laio-0.10.1-x86_64-linux.tgz"
      sha256 "020675df9ba490ce1360443fe9f188fafcad8fc35fc32463558638645425f5d9"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.1/laio-0.10.1-aarch64-linux.tgz"
      sha256 "ddafda21702a4a35269ed5cdbfea47991d64d37f6a4ac6e7926571fb750b8d64"
    end
  end

  def install
    bin.install "laio"
  end
end
