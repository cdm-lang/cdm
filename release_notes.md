# cdm-plugin-sql v0.1.0

## Installation

Download the WASM file and verify its integrity:

```bash
# Download the plugin
curl -LO https://github.com/cdm-lang/cdm/releases/download/cdm-plugin-sql-v0.1.0/cdm_plugin_sql.wasm

# Download the checksum
curl -LO https://github.com/cdm-lang/cdm/releases/download/cdm-plugin-sql-v0.1.0/cdm_plugin_sql.wasm.sha256

# Verify the checksum
echo "$(cat cdm_plugin_sql.wasm.sha256)  cdm_plugin_sql.wasm" | shasum -a 256 -c
```

## Checksum

SHA256: `$(cat crates/cdm-plugin-sql/target/wasm32-wasip1/release/cdm_plugin_sql.wasm.sha256)`

## Plugin Info

- **Plugin Name**: cdm-plugin-sql
- **Version**: v0.1.0
- **WASM Target**: wasm32-wasip1
