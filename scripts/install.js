const path = require('path');
const { ensureBinary } = require('./util');

(async () => {
  const binDir = path.join(__dirname, '..', 'bin');
  try {
    await ensureBinary(binDir);
  } catch (err) {
    console.error(err.message || err);
    process.exit(1);
  }
})();
