import os
import platform
import stat
import subprocess
import sys
import tarfile
import tempfile
import urllib.request
import zipfile
from pathlib import Path

VERSION = "0.5.1"
BASE_URL = os.environ.get(
    "GITKAT_RELEASE_BASE",
    f"https://github.com/Aureuma/GitKat/releases/download/v{VERSION}",
)


def main() -> int:
    binary = ensure_binary()
    args = [binary, *sys.argv[1:]]
    if os.name == "nt":
        return subprocess.call(args)
    os.execv(binary, args)
    return 1


def ensure_binary() -> str:
    target = detect_target()
    cache_dir = Path(os.environ.get("GITKAT_CACHE_DIR", Path.home() / ".cache" / "gitkat"))
    version_dir = cache_dir / VERSION
    binary_name = "gk.exe" if os.name == "nt" else "gk"
    binary_path = version_dir / binary_name
    if binary_path.exists():
        return str(binary_path)

    version_dir.mkdir(parents=True, exist_ok=True)
    archive_path = download_asset(target, version_dir)
    extract_archive(archive_path, version_dir)

    if not binary_path.exists():
        raise RuntimeError(f"Expected binary at {binary_path}")

    if os.name != "nt":
        binary_path.chmod(binary_path.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
    return str(binary_path)


def detect_target() -> str:
    system = platform.system().lower()
    machine = platform.machine().lower()

    if machine in {"x86_64", "amd64"}:
        arch = "x86_64"
    elif machine in {"aarch64", "arm64"}:
        arch = "aarch64"
    else:
        raise RuntimeError(f"Unsupported architecture: {machine}")

    if system == "darwin":
        return f"{arch}-apple-darwin"
    if system == "linux":
        return f"{arch}-unknown-linux-gnu"
    if system == "windows":
        return f"{arch}-pc-windows-msvc"

    raise RuntimeError(f"Unsupported operating system: {system}")


def asset_name(target: str) -> str:
    ext = "zip" if os.name == "nt" else "tar.gz"
    return f"gitkat-v{VERSION}-{target}.{ext}"


def download_asset(target: str, dest_dir: Path) -> Path:
    name = asset_name(target)
    url = f"{BASE_URL}/{name}"
    dest = dest_dir / name
    if dest.exists():
        return dest

    with urllib.request.urlopen(url) as response:
        if response.status != 200:
            raise RuntimeError(f"Failed to download {url}: {response.status}")
        with tempfile.NamedTemporaryFile(delete=False) as tmp:
            tmp.write(response.read())
            tmp_path = Path(tmp.name)
    tmp_path.replace(dest)
    return dest


def extract_archive(archive: Path, dest_dir: Path) -> None:
    if archive.suffix == ".zip":
        with zipfile.ZipFile(archive, "r") as zf:
            zf.extractall(dest_dir)
        return

    with tarfile.open(archive, "r:gz") as tf:
        tf.extractall(dest_dir)
