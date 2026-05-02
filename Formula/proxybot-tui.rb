class ProxybotTui < Formula
  desc "ProxyBot TUI — HTTPS MITM proxy terminal UI"
  homepage "https://github.com/mbpz/proxybot"
  license "MIT"
  version "v0.4.2"

  if OS.mac? && Hardware::CPU.type == :arm64
    url "https://github.com/mbpz/proxybot/releases/download/tui-v0.4.2/proxybot-tui-arm64"
    sha256 "f23af267022294d9482b7036c83761163eac3e8165452c63892d9f0563e9cca8"
  elsif OS.mac? && Hardware::CPU.type == :intel
    url "https://github.com/mbpz/proxybot/releases/download/tui-v0.4.2/proxybot-tui-x86_64"
    sha256 "07df5fd82d239232e52fba318ad295c82452c360e166f3297a5cfce9a80fdff5"
  end

  def install
    bin.install "proxybot-tui" => "proxybot-tui"
  end
end
