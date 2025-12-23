class Gitkat < Formula
  desc "GitKat: bulk Git repository utilities"
  homepage "https://github.com/Aureuma/GitKat"
  url "https://github.com/Aureuma/GitKat/archive/refs/tags/v0.5.1.tar.gz"
  sha256 "d1bc5ea903055aae7f118d3fd8fe4429f9875891486f9c0cb3e7cc253f7de3c5"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "crates/gitkat-cli")
  end

  test do
    assert_match "GitKat", shell_output("#{bin}/gk --help")
  end
end
