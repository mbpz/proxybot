class ProxybotTui < Formula
  desc "ProxyBot TUI — HTTPS MITM proxy terminal UI"
  homepage "https://github.com/mbpz/proxybot"
  license "MIT"
  version "v0.4.2"

  if OS.mac? && Hardware::CPU.type == :arm64
    url "https://github.com/mbpz/proxybot/releases/download/tui-v0.4.2/proxybot-tui-arm64"
    sha256 ""
  elsif OS.mac? && Hardware::CPU.type == :intel
    url "https://github.com/mbpz/proxybot/releases/download/tui-v0.4.2/proxybot-tui-x86_64"
    sha256 ""
  end

  def install
    bin.install "proxybot-tui" => "proxybot-tui"
  end
end
