class ProxybotTui < Formula
  desc "Terminal UI for ProxyBot HTTPS MITM proxy"
  homepage "https://github.com/mbpz/proxybot"
  url "https://github.com/mbpz/proxybot/releases/download/v0.1.0/proxybot-tui"
  sha256 "f0bfd83780bf6e59a62b205c686652feaaec31ff74614fe223774104c6d78b6a"
  license "MIT"
  version "0.1.0"

  def install
    bin.install "proxybot-tui"
  end
end
