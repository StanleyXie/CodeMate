class Codemate < Formula
  desc "AI-powered code intelligence and graph analysis tool"
  homepage "https://github.com/StanleyXie/CodeMate"
  version "0.2.0"

  if OS.mac?
    if Hardware::CPU.arm?
      url "https://github.com/StanleyXie/CodeMate/releases/download/v0.2.0/codemate-aarch64-apple-darwin.tar.gz"
      sha256 "f47fd5d2c8d07530e4db876660a96b7ee8e7e19f8a4401abdeec215b97decacf"
    else
      url "https://github.com/StanleyXie/CodeMate/releases/download/v0.2.0/codemate-x86_64-apple-darwin.tar.gz"
      sha256 "d7a5d72d23f9898d6e1179f275431a5d709945603597559867e812f94d65e3b0"
    end
  elsif OS.linux?
    if Hardware::CPU.intel?
      url "https://github.com/StanleyXie/CodeMate/releases/download/v0.2.0/codemate-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "1f4eff626cb44a0cebb05c5220155b5111160467ea18b66772af604ffeaabc0c"
    end
  end

  def install
    bin.install "codemate"
    bin.install "codemate-server"
  end

  test do
    system "#{bin}/codemate", "--version"
    system "#{bin}/codemate-server", "--version"
  end
end
