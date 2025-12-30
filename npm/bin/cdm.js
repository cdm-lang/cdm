#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

const BINARY_NAME = process.platform === 'win32' ? 'cdm.exe' : 'cdm';
const BINARY_PATH = path.join(__dirname, BINARY_NAME);

// Check if binary exists
if (!fs.existsSync(BINARY_PATH)) {
  console.error('Error: CDM binary not found.');
  console.error('This usually means the installation did not complete successfully.');
  console.error('Try reinstalling the package: npm install @cdm-lang/cdm');
  process.exit(1);
}

// Spawn the binary with all arguments
const child = spawn(BINARY_PATH, process.argv.slice(2), {
  stdio: 'inherit',
  windowsHide: true,
});

child.on('exit', (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
  } else {
    process.exit(code);
  }
});

// Handle process termination
process.on('SIGINT', () => {
  child.kill('SIGINT');
  child.kill('SIGTERM');
});
