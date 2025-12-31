class Codemate < Formula
  desc "AI-powered code intelligence and graph analysis tool"
  homepage "https://github.com/StanleyXie/CodeMate"
  version "0.2.3"

  if OS.mac?
    if Hardware::CPU.arm?
      url "https://github.com/StanleyXie/CodeMate/releases/download/v0.2.3/codemate-aarch64-apple-darwin.tar.gz"
      sha256 "0e85df780ad60b1a4ab8c3341e862ea35b2d037a23de6d25fce7076368afb128"
    else
      url "https://github.com/StanleyXie/CodeMate/releases/download/v0.2.3/codemate-x86_64-apple-darwin.tar.gz"
      sha256 "822b3826b480c54dfb23f15272aa9277d29bbabda45d8ee8813e3b025f24626f"
    end
  elsif OS.linux?
    if Hardware::CPU.intel?
      url "https://github.com/StanleyXie/CodeMate/releases/download/v0.2.3/codemate-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "522a5a28f29ebf4c8a4135ea40db26eaefcaa82d671cd7d49dc7d699ec85662f"
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
