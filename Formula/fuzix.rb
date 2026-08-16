class Fuzix < Formula
  desc "Modern native CLI developer tool and SDK for FUZIX OS"
  homepage "https://github.com/notKitory/fuzix-sh"
  url "https://github.com/notKitory/fuzix-sh/archive/refs/heads/main.tar.gz"
  version "0.2.0"
  head "https://github.com/notKitory/fuzix-sh.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "Modern native CLI developer tool", shell_output("#{bin}/fuzix --help")
  end
end
