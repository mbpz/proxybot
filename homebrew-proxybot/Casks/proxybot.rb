cask "proxybot" do
  version "1.3.0"
  sha256 arm:   "REPLACE_WITH_ARM64_SHA256",
         intel: "REPLACE_WITH_X64_SHA256"

  url "https://github.com/mbpz/proxybot/releases/download/v#{version}/ProxyBot-#{version}-mac-#{Hardware::CPU.arch}.zip",
      verified: "github.com/mbpz/proxybot/"
  name "ProxyBot"
  desc "macOS HTTPS MITM proxy for developers — intercept and classify mobile app traffic"
  homepage "https://github.com/mbpz/proxybot"

  depends_on macos: ">= :big_sur"

  app "ProxyBot.app"

  zap trash: [
    "~/Library/Application Support/com.proxybot.app",
    "~/.proxybot",
  ]
end
