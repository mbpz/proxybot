class ProxybotTui < Formula
  desc "ProxyBot TUI — HTTPS MITM proxy terminal UI for developers"
  homepage "https://github.com/mbpz/proxybot"
  license "MIT"
  version "1.2.0"

  on_macos do
    if Hardware::CPU.arm64?
      url "https://github.com/mbpz/proxybot/releases/download/v1.2.0/proxybot-tui-1.2.0-mac-arm64"
      sha256 "e6812dc3105a56fbb2267939f44e41dcbb310274631e4352da19b2ac45670ad8"
    else
      url "https://github.com/mbpz/proxybot/releases/download/v1.2.0/proxybot-tui-1.2.0-mac-x64"
      sha256 "TODO: compute after first release - run on Intel Mac"
    end
  end

  def install
    bin.install "proxybot-tui"
  end
end