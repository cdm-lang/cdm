#!/usr/bin/env node

const https = require('https');
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { promisify } = require('util');
const { pipeline } = require('stream');

const streamPipeline = promisify(pipeline);

const MANIFEST_URL = 'https://raw.githubusercontent.com/cdm-lang/cdm/main/cli-releases.json';
const BINARY_DIR = path.join(__dirname, '..', 'bin');
const BINARY_NAME = process.platform === 'win32' ? 'cdm.exe' : 'cdm';
const BINARY_PATH = path.join(BINARY_DIR, BINARY_NAME);

function getPlatformInfo() {
  const platform = process.platform;
  const arch = process.arch;

  const platformMap = {
    darwin: {
      x64: 'x86_64-apple-darwin',
      arm64: 'aarch64-apple-darwin',
    },
    linux: {
      x64: 'x86_64-unknown-linux-gnu',
      arm64: 'aarch64-unknown-linux-gnu',
    },
    win32: {
      x64: 'x86_64-pc-windows-msvc.exe',
    },
  };

  if (!platformMap[platform] || !platformMap[platform][arch]) {
    throw new Error(
      `Unsupported platform: ${platform}-${arch}. ` +
      `CDM supports: macOS (x64, arm64), Linux (x64, arm64), Windows (x64)`
    );
  }

  return platformMap[platform][arch];
}

function fetchJSON(url) {
  return new Promise((resolve, reject) => {
    https.get(url, (res) => {
      if (res.statusCode === 302 || res.statusCode === 301) {
        return fetchJSON(res.headers.location).then(resolve).catch(reject);
      }

      if (res.statusCode !== 200) {
        return reject(new Error(`Failed to fetch ${url}: ${res.statusCode}`));
      }

      let data = '';
      res.on('data', (chunk) => data += chunk);
      res.on('end', () => {
        try {
          resolve(JSON.parse(data));
        } catch (err) {
          reject(err);
        }
      });
    }).on('error', reject);
  });
}

function downloadFile(url, destPath) {
  return new Promise((resolve, reject) => {
    https.get(url, (res) => {
      if (res.statusCode === 302 || res.statusCode === 301) {
        return downloadFile(res.headers.location, destPath).then(resolve).catch(reject);
      }

      if (res.statusCode !== 200) {
        return reject(new Error(`Failed to download ${url}: ${res.statusCode}`));
      }

      const fileStream = fs.createWriteStream(destPath);
      streamPipeline(res, fileStream)
        .then(resolve)
        .catch(reject);
    }).on('error', reject);
  });
}

function verifyChecksum(filePath, expectedChecksum) {
  const fileBuffer = fs.readFileSync(filePath);
  const hash = crypto.createHash('sha256');
  hash.update(fileBuffer);
  const actualChecksum = hash.digest('hex');

  if (actualChecksum !== expectedChecksum) {
    throw new Error(
      `Checksum verification failed!\n` +
      `Expected: ${expectedChecksum}\n` +
      `Actual:   ${actualChecksum}`
    );
  }
}

async function install() {
  console.log('Installing CDM CLI...');

  try {
    // Get platform info
    const platformKey = getPlatformInfo();
    console.log(`Platform: ${platformKey}`);

    // Fetch manifest
    console.log('Fetching release manifest...');
    const manifest = await fetchJSON(MANIFEST_URL);

    const latestVersion = manifest.latest;
    console.log(`Latest version: ${latestVersion}`);

    const release = manifest.releases[latestVersion];
    if (!release) {
      throw new Error(`Release ${latestVersion} not found in manifest`);
    }

    const platformInfo = release.platforms[platformKey];
    if (!platformInfo) {
      throw new Error(`Platform ${platformKey} not found in release ${latestVersion}`);
    }

    const { url, checksum } = platformInfo;
    const expectedChecksum = checksum.replace('sha256:', '');

    // Create bin directory
    if (!fs.existsSync(BINARY_DIR)) {
      fs.mkdirSync(BINARY_DIR, { recursive: true });
    }

    // Download binary
    console.log(`Downloading from ${url}...`);
    await downloadFile(url, BINARY_PATH);

    // Verify checksum
    console.log('Verifying checksum...');
    verifyChecksum(BINARY_PATH, expectedChecksum);

    // Make executable (Unix only)
    if (process.platform !== 'win32') {
      fs.chmodSync(BINARY_PATH, 0o755);
    }

    console.log('CDM CLI installed successfully!');
    console.log(`Binary location: ${BINARY_PATH}`);
    console.log('\nRun "cdm --help" to get started.');

  } catch (error) {
    console.error('Installation failed:', error.message);
    process.exit(1);
  }
}

install();
