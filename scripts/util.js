const fs = require('fs');
const https = require('https');
const os = require('os');
const path = require('path');
const { promisify } = require('util');
const AdmZip = require('adm-zip');
const tar = require('tar');

const pipeline = promisify(require('stream').pipeline);
const pkg = require('../package.json');

const VERSION = pkg.version;
const BASE_URL = process.env.GITKAT_RELEASE_BASE || `https://github.com/Aureuma/GitKat/releases/download/v${VERSION}`;

function detectTarget() {
  const platform = process.platform;
  const arch = process.arch;

  let cpu;
  if (arch === 'x64') {
    cpu = 'x86_64';
  } else if (arch === 'arm64') {
    cpu = 'aarch64';
  } else {
    throw new Error(`Unsupported architecture: ${arch}`);
  }

  if (platform === 'darwin') {
    return `${cpu}-apple-darwin`;
  }
  if (platform === 'linux') {
    return `${cpu}-unknown-linux-gnu`;
  }
  if (platform === 'win32') {
    return `${cpu}-pc-windows-msvc`;
  }

  throw new Error(`Unsupported platform: ${platform}`);
}

function assetName(target) {
  const ext = process.platform === 'win32' ? 'zip' : 'tar.gz';
  return `gitkat-v${VERSION}-${target}.${ext}`;
}

async function download(url, destPath) {
  await new Promise((resolve, reject) => {
    https.get(url, (res) => {
      if (res.statusCode !== 200) {
        reject(new Error(`Failed to download ${url}: ${res.statusCode}`));
        res.resume();
        return;
      }
      pipeline(res, fs.createWriteStream(destPath))
        .then(resolve)
        .catch(reject);
    }).on('error', reject);
  });
}

async function extract(archivePath, destDir) {
  if (archivePath.endsWith('.zip')) {
    const zip = new AdmZip(archivePath);
    zip.extractAllTo(destDir, true);
    return;
  }
  await tar.x({ file: archivePath, cwd: destDir });
}

function binaryName() {
  return process.platform === 'win32' ? 'gk.exe' : 'gk';
}

function binaryPath(binDir) {
  return path.join(binDir, binaryName());
}

async function ensureBinary(binDir) {
  const target = detectTarget();
  const binPath = binaryPath(binDir);
  if (fs.existsSync(binPath)) {
    return binPath;
  }

  fs.mkdirSync(binDir, { recursive: true });

  const name = assetName(target);
  const url = `${BASE_URL}/${name}`;
  const cacheDir = path.join(os.homedir(), '.cache', 'gitkat', VERSION);
  const archivePath = path.join(cacheDir, name);
  fs.mkdirSync(cacheDir, { recursive: true });

  if (!fs.existsSync(archivePath)) {
    await download(url, archivePath);
  }

  await extract(archivePath, binDir);

  if (!fs.existsSync(binPath)) {
    throw new Error(`Expected binary at ${binPath}`);
  }

  if (process.platform !== 'win32') {
    fs.chmodSync(binPath, 0o755);
  }

  return binPath;
}

module.exports = {
  ensureBinary,
  binaryPath,
};
