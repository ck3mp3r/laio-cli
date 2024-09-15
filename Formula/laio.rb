class Laio < Formula
  desc "laio - a simple, flexbox-inspired, layout & session manager for tmux."
  homepage "https://laio.sh"
  version "0.10.6"

  depends_on "tmux"
  
  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.6/laio-0.10.6-x86_64-darwin.tgz"
      sha256 "a9377a29b2597b823775ee5b54a19bcac9658c82f9347fe55a1e586f7c31f153"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.6/laio-0.10.6-aarch64-darwin.tgz"
      sha256 "c757c39a8e347b03537005e2a8907c3feb9b5f15fc7397e5bd91524071c18b68"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.6/laio-0.10.6-x86_64-linux.tgz"
      sha256 "76a44f23e7937fd7c4686a5f9cc46d91bf3f1ac3ad635111ec2902699caf3ff4"
    elsif Hardware::CPU.arm?
      url "https://github.com/ck3mp3r/laio-cli/releases/download/v0.10.6/laio-0.10.6-aarch64-linux.tgz"
      sha256 "e23552f855b735188404860a56d6ada6dd610947a72a24e5ea356a2ce7c15cde"
    end
  end

  def install
    bin.install "laio"
  end
end
