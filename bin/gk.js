#!/usr/bin/env node
const path = require('path');
const { spawnSync } = require('child_process');
const { ensureBinary, binaryPath } = require('../scripts/util');

(async () => {
  const binDir = path.join(__dirname);
  try {
    await ensureBinary(binDir);
  } catch (err) {
    console.error(err.message || err);
    process.exit(1);
  }

  const binPath = binaryPath(binDir);
  const result = spawnSync(binPath, process.argv.slice(2), { stdio: 'inherit' });
  process.exit(result.status === null ? 1 : result.status);
})();
