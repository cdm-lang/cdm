# CDM (Contextual Data Models)

**CDM** is a language for defining data models and generating code across multiple platforms.

## Installation

### Quick Install

#### Unix/Linux/macOS

```bash
curl -fsSL https://raw.githubusercontent.com/cdm-lang/cdm/main/install.sh | sh
```

#### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/cdm-lang/cdm/main/install.ps1 | iex
```

### Alternative Installation Methods

#### via npm

```bash
# Install globally
npm install -g @cdm-lang/cli

# Or in a project
npm install --save-dev @cdm-lang/cli
```

See the [npm package documentation](npm/README.md) for more details.

#### Manual Download

Download the appropriate binary for your platform from the [releases page](https://github.com/cdm-lang/cdm/releases):

<details>
<summary>macOS (Apple Silicon)</summary>

```bash
curl -LO https://github.com/cdm-lang/cdm/releases/download/cdm-cli-v0.1.9/cdm-aarch64-apple-darwin
curl -LO https://github.com/cdm-lang/cdm/releases/download/cdm-cli-v0.1.9/cdm-aarch64-apple-darwin.sha256
echo "$(cat cdm-aarch64-apple-darwin.sha256)  cdm-aarch64-apple-darwin" | shasum -a 256 -c
chmod +x cdm-aarch64-apple-darwin
sudo mv cdm-aarch64-apple-darwin /usr/local/bin/cdm
```
</details>

<details>
<summary>macOS (Intel)</summary>

```bash
curl -LO https://github.com/cdm-lang/cdm/releases/download/cdm-cli-v0.1.9/cdm-x86_64-apple-darwin
curl -LO https://github.com/cdm-lang/cdm/releases/download/cdm-cli-v0.1.9/cdm-x86_64-apple-darwin.sha256
echo "$(cat cdm-x86_64-apple-darwin.sha256)  cdm-x86_64-apple-darwin" | shasum -a 256 -c
chmod +x cdm-x86_64-apple-darwin
sudo mv cdm-x86_64-apple-darwin /usr/local/bin/cdm
```
</details>

<details>
<summary>Linux (x86_64)</summary>

```bash
curl -LO https://github.com/cdm-lang/cdm/releases/download/cdm-cli-v0.1.9/cdm-x86_64-unknown-linux-gnu
curl -LO https://github.com/cdm-lang/cdm/releases/download/cdm-cli-v0.1.9/cdm-x86_64-unknown-linux-gnu.sha256
echo "$(cat cdm-x86_64-unknown-linux-gnu.sha256)  cdm-x86_64-unknown-linux-gnu" | sha256sum -c
chmod +x cdm-x86_64-unknown-linux-gnu
sudo mv cdm-x86_64-unknown-linux-gnu /usr/local/bin/cdm
```
</details>

<details>
<summary>Linux (ARM64)</summary>

```bash
curl -LO https://github.com/cdm-lang/cdm/releases/download/cdm-cli-v0.1.9/cdm-aarch64-unknown-linux-gnu
curl -LO https://github.com/cdm-lang/cdm/releases/download/cdm-cli-v0.1.9/cdm-aarch64-unknown-linux-gnu.sha256
echo "$(cat cdm-aarch64-unknown-linux-gnu.sha256)  cdm-aarch64-unknown-linux-gnu" | sha256sum -c
chmod +x cdm-aarch64-unknown-linux-gnu
sudo mv cdm-aarch64-unknown-linux-gnu /usr/local/bin/cdm
```
</details>

<details>
<summary>Windows (x86_64)</summary>

```powershell
Invoke-WebRequest -Uri "https://github.com/cdm-lang/cdm/releases/download/cdm-cli-v0.1.9/cdm-x86_64-pc-windows-msvc.exe" -OutFile "cdm.exe"
Invoke-WebRequest -Uri "https://github.com/cdm-lang/cdm/releases/download/cdm-cli-v0.1.9/cdm-x86_64-pc-windows-msvc.exe.sha256" -OutFile "cdm.exe.sha256"
# Move to a directory in your PATH
```
</details>

#### Build from Source

```bash
git clone https://github.com/cdm-lang/cdm.git
cd cdm
cargo build --release --bin cdm
# Binary will be at target/release/cdm
```

## Usage

After installation, verify CDM is working:

```bash
cdm --version
cdm --help
```

### Available Commands

```
cdm <COMMAND>

Commands:
  validate     Validate a CDM file
  build        Build output files from a CDM schema using configured plugins
  migrate      Generate migration files from schema changes
  plugin       Plugin management commands
  format       Format CDM files and optionally assign entity IDs
  update       Update CDM CLI to a different version
  completions  Generate shell completions
  help         Print this message or the help of the given subcommand(s)
```

### Shell Completions

CDM supports tab completion for bash, zsh, fish, PowerShell, and elvish.

#### Automatic Installation

Shell completions are automatically installed when using the install scripts:
- Unix/Linux/macOS: `curl -fsSL https://raw.githubusercontent.com/cdm-lang/cdm/main/install.sh | sh`
- Windows: `irm https://raw.githubusercontent.com/cdm-lang/cdm/main/install.ps1 | iex`

#### Manual Installation

If you installed via npm or manual download, you can generate completions manually:

**Bash:**
```bash
cdm completions bash > ~/.local/share/bash-completion/completions/cdm
# Or for older systems:
cdm completions bash > ~/.bash_completion.d/cdm
```

**Zsh:**
```bash
mkdir -p ~/.zsh/completions
cdm completions zsh > ~/.zsh/completions/_cdm

# Add to ~/.zshrc:
fpath=(~/.zsh/completions $fpath)
autoload -Uz compinit && compinit
```

**Fish:**
```bash
cdm completions fish > ~/.config/fish/completions/cdm.fish
```

**PowerShell:**
```powershell
cdm completions powershell | Out-File -FilePath "$HOME\Documents\PowerShell\cdm-completion.ps1"

# Add to your PowerShell profile:
. "$HOME\Documents\PowerShell\cdm-completion.ps1"
```

**Elvish:**
```bash
cdm completions elvish > ~/.elvish/lib/cdm-completions.elv
```

## Updating

### If installed via install script

The CDM CLI includes a built-in update command:

```bash
cdm update
```

### If installed via npm

Update using npm:

```bash
# Global installation
npm update -g @cdm-lang/cli

# Local installation
npm update @cdm-lang/cli
```

See the [npm package documentation](npm/README.md#updating) for more details.

## Uninstalling

### If installed via install script

**Using the uninstall command:**
```bash
cdm uninstall
```

**Or using the uninstall script:**

Unix/Linux/macOS:
```bash
curl -fsSL https://raw.githubusercontent.com/cdm-lang/cdm/main/uninstall.sh | sh
```

Windows (PowerShell):
```powershell
irm https://raw.githubusercontent.com/cdm-lang/cdm/main/uninstall.ps1 | iex
```

### If installed via npm

```bash
# Global installation
npm uninstall -g @cdm-lang/cli

# Local installation
npm uninstall @cdm-lang/cli
```

### Manual Uninstallation

If you need to manually uninstall:

**Unix/Linux/macOS:**
```bash
# Remove the binary
rm -rf ~/.cdm

# Remove completions (choose your shell)
rm -f ~/.local/share/bash-completion/completions/cdm  # Bash
rm -f ~/.zsh/completions/_cdm                          # Zsh
rm -f ~/.config/fish/completions/cdm.fish              # Fish

# Remove PATH modification from your shell profile (~/.bashrc, ~/.zshrc, etc.)
# Then restart your shell
```

**Windows:**
```powershell
# Remove the binary
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\cdm"

# Remove completions
Remove-Item "$HOME\Documents\PowerShell\cdm-completion.ps1"

# Remove from PATH via System Settings
```

## Supported Platforms

- macOS (Intel x64, Apple Silicon arm64)
- Linux (x64, ARM64)
- Windows (x64)

## Project Structure

```
cdm/
├── crates/
│   ├── cdm/                    # Main CLI crate
│   ├── cdm-plugin-interface/   # Plugin system interface
│   ├── cdm-plugin-docs/        # Documentation generator plugin
│   ├── cdm-plugin-sql/         # SQL generator plugin
│   ├── cdm-plugin-typescript/  # TypeScript generator plugin
│   ├── cdm-lsp/               # Language Server Protocol implementation
│   ├── cdm-utils/             # Shared utilities
│   ├── cdm-json-validator/    # JSON schema validator
│   └── grammar/               # Tree-sitter grammar
├── editors/
│   └── vscode-cdm/            # VSCode extension
├── npm/                       # npm package distribution
├── examples/                  # Example CDM schemas
└── specs/                     # Specifications and documentation
```

## Development

### Prerequisites

- Rust (latest stable)
- Node.js (for npm package and VSCode extension)
- Just (task runner)

### Setup

```bash
# Install just
cargo install just

# Setup the project
just setup

# Build the project
just build
```

### Available Just Commands

Run `just --list` to see all available commands.

## License

MPL-2.0

## Links

- [Releases](https://github.com/cdm-lang/cdm/releases)
- [Issue Tracker](https://github.com/cdm-lang/cdm/issues)
- [npm Package](https://www.npmjs.com/package/@cdm-lang/cli)

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.
