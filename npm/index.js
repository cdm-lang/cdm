#!/usr/bin/env node

const path = require('path');
const { spawnSync } = require('child_process');

const BINARY_NAME = process.platform === 'win32' ? 'cdm.exe' : 'cdm';
const BINARY_PATH = path.join(__dirname, 'bin', BINARY_NAME);

function getBinaryPath() {
  return BINARY_PATH;
}

function run(args = []) {
  const result = spawnSync(BINARY_PATH, args, {
    stdio: 'inherit',
    windowsHide: true,
  });

  if (result.error) {
    throw result.error;
  }

  return result.status;
}

module.exports = {
  getBinaryPath,
  run,
};
