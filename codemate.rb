class Codemate < Formula
  desc "AI-powered code intelligence and graph analysis tool"
  homepage "https://github.com/StanleyXie/CodeMate"
  version "0.2.0"

  if OS.mac?
    if Hardware::CPU.arm?
      url "https://github.com/StanleyXie/CodeMate/releases/download/v0.2.0/codemate-aarch64-apple-darwin.tar.gz"
      sha256 "5c4e31ea9fb428fc103795757b8cb4545d94657d943a8ce32ee01308b9dad7b9"
    else
      url "https://github.com/StanleyXie/CodeMate/releases/download/v0.2.0/codemate-x86_64-apple-darwin.tar.gz"
      sha256 "e2a589294ec9a335780ca2758e374f863df108e89cf12903d9bab2b397c8fc27"
    end
  elsif OS.linux?
    if Hardware::CPU.intel?
      url "https://github.com/StanleyXie/CodeMate/releases/download/v0.2.0/codemate-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "2e35c52d7af483996d6cc109d9ff34e20590b0029fc06a0792d5549610b1aafb"
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
