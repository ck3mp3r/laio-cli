class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.11.4"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.4/laio-0.11.4-x86_64-darwin.tgz"
      sha256 "8e17f01905ab3f841b34637ee5ca4e0cd9acf78ff800f711a1b6887c5b1ec86e"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.4/laio-0.11.4-aarch64-darwin.tgz"
      sha256 "51dd16072365a84a95af4fdaeadfd0950767a36009ec5eca586a1281f7e1f8da"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.4/laio-0.11.4-x86_64-linux.tgz"
      sha256 "0dd40176bf80ea5e1e685a41c8e4a8a5aa21ea837414eb478f83f65bf1c8d455"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.11.4/laio-0.11.4-aarch64-linux.tgz"
      sha256 "badb0aade7cfd3bb7771180b8d0522ddae32d43f0f6444a4996f48dd2bcc2e18"
    end
  end

  def install
    bin.install "laio"
  end
end
