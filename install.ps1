# CDM CLI Installer for Windows PowerShell
# Usage: irm https://raw.githubusercontent.com/cdm-lang/cdm/main/install.ps1 | iex

$ErrorActionPreference = 'Stop'

$Repo = "cdm-lang/cdm"
$ManifestUrl = "https://raw.githubusercontent.com/cdm-lang/cdm/main/cli-releases.json"
$InstallDir = if ($env:CDM_INSTALL_DIR) { $env:CDM_INSTALL_DIR } else { "$env:LOCALAPPDATA\cdm\bin" }
$BinaryName = "cdm.exe"

# Helper functions
function Write-ColorOutput {
    param(
        [string]$Message,
        [string]$Color = 'White'
    )
    Write-Host $Message -ForegroundColor $Color
}

function Write-Info {
    param([string]$Message)
    Write-ColorOutput "==> $Message" -Color Green
}

function Write-Warn {
    param([string]$Message)
    Write-ColorOutput "Warning: $Message" -Color Yellow
}

function Write-Error {
    param([string]$Message)
    Write-ColorOutput "Error: $Message" -Color Red
    exit 1
}

# Detect platform
function Get-Platform {
    $arch = $env:PROCESSOR_ARCHITECTURE

    switch ($arch) {
        "AMD64" { return "x86_64-pc-windows-msvc.exe" }
        "ARM64" { Write-Error "ARM64 Windows is not currently supported" }
        default { Write-Error "Unsupported architecture: $arch" }
    }
}

# Fetch latest version from manifest
function Get-LatestVersion {
    try {
        $manifest = Invoke-RestMethod -Uri $ManifestUrl -UseBasicParsing
        return $manifest.latest
    }
    catch {
        Write-Error "Failed to fetch latest version: $_"
    }
}

# Download file with progress
function Download-File {
    param(
        [string]$Url,
        [string]$OutputPath
    )

    try {
        Write-Info "Downloading from $Url..."
        $ProgressPreference = 'SilentlyContinue'
        Invoke-WebRequest -Uri $Url -OutFile $OutputPath -UseBasicParsing
        $ProgressPreference = 'Continue'
    }
    catch {
        Write-Error "Failed to download file: $_"
    }
}

# Verify checksum
function Test-Checksum {
    param(
        [string]$FilePath,
        [string]$ExpectedChecksum
    )

    try {
        $hash = Get-FileHash -Path $FilePath -Algorithm SHA256
        $actualChecksum = $hash.Hash.ToLower()

        if ($actualChecksum -ne $ExpectedChecksum) {
            Write-Error "Checksum verification failed!`nExpected: $ExpectedChecksum`nActual:   $actualChecksum"
        }

        Write-Info "Checksum verified successfully"
    }
    catch {
        Write-Error "Failed to verify checksum: $_"
    }
}

# Add directory to PATH
function Add-ToPath {
    param([string]$Directory)

    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")

    if ($userPath -notlike "*$Directory*") {
        Write-Info "Adding $Directory to PATH..."
        $newPath = "$userPath;$Directory"
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")

        # Update current session PATH
        $env:Path = "$env:Path;$Directory"

        Write-Info "PATH updated successfully"
        Write-Warn "You may need to restart your terminal for the PATH changes to take effect"
        return $true
    }

    return $false
}

# Main installation function
function Install-CDM {
    Write-Info "Installing CDM CLI..."
    Write-Host ""

    # Detect platform
    $platform = Get-Platform
    Write-Info "Detected platform: $platform"

    # Get latest version
    $version = Get-LatestVersion
    if (-not $version) {
        Write-Error "Failed to fetch latest version"
    }
    Write-Info "Latest version: $version"

    # Construct download URLs
    $tag = "cdm-cli-v$version"
    $binaryUrl = "https://github.com/$Repo/releases/download/$tag/cdm-$platform"
    $checksumUrl = "https://github.com/$Repo/releases/download/$tag/cdm-$platform.sha256"

    # Create temporary directory
    $tmpDir = New-Item -ItemType Directory -Path (Join-Path $env:TEMP "cdm-install-$(Get-Random)")

    try {
        $tmpBinary = Join-Path $tmpDir $BinaryName
        $tmpChecksum = Join-Path $tmpDir "$BinaryName.sha256"

        # Download binary
        Write-Info "Downloading CDM CLI v$version..."
        Download-File -Url $binaryUrl -OutputPath $tmpBinary

        # Download checksum
        Write-Info "Downloading checksum..."
        Download-File -Url $checksumUrl -OutputPath $tmpChecksum

        # Verify checksum
        Write-Info "Verifying checksum..."
        $expectedChecksum = (Get-Content $tmpChecksum).Trim()
        Test-Checksum -FilePath $tmpBinary -ExpectedChecksum $expectedChecksum

        # Create installation directory
        if (-not (Test-Path $InstallDir)) {
            New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        }

        # Move binary to installation directory
        $installPath = Join-Path $InstallDir $BinaryName

        # Remove old binary if it exists
        if (Test-Path $installPath) {
            Remove-Item $installPath -Force
        }

        Move-Item -Path $tmpBinary -Destination $installPath -Force

        Write-Host ""
        Write-Info "CDM CLI v$version installed successfully!"
        Write-Host ""
        Write-Info "Binary location: $installPath"
        Write-Host ""

        # Install shell completions
        Install-Completions -BinaryPath $installPath

        # Add to PATH
        $addedToPath = Add-ToPath -Directory $InstallDir

        Write-Host ""
        if ($addedToPath) {
            Write-Info "You can now run: cdm --help"
            Write-Warn "Note: You may need to restart your terminal for PATH changes to take effect"
        }
        else {
            Write-Info "Installation directory is already in PATH"
            Write-Info "You can now run: cdm --help"
        }
    }
    finally {
        # Clean up temporary directory
        if (Test-Path $tmpDir) {
            Remove-Item $tmpDir -Recurse -Force
        }
    }
}

# Install PowerShell completions
function Install-Completions {
    param([string]$BinaryPath)

    try {
        # Generate PowerShell completion script
        $completionScript = & $BinaryPath completions powershell 2>$null

        if ($LASTEXITCODE -eq 0 -and $completionScript) {
            # Determine PowerShell profile directory
            $profileDir = Split-Path -Parent $PROFILE
            $completionFile = Join-Path $profileDir "cdm-completion.ps1"

            # Create profile directory if it doesn't exist
            if (-not (Test-Path $profileDir)) {
                New-Item -ItemType Directory -Path $profileDir -Force | Out-Null
            }

            # Save completion script
            $completionScript | Out-File -FilePath $completionFile -Encoding UTF8 -Force

            Write-Info "Installed PowerShell completions to $completionFile"
            Write-Host ""
            Write-Info "To enable completions, add this to your PowerShell profile ($PROFILE):"
            Write-Host ""
            Write-Host "    . `"$completionFile`"" -ForegroundColor Cyan
            Write-Host ""
            Write-Info "Or run this command to add it automatically:"
            Write-Host ""
            Write-Host "    Add-Content -Path `$PROFILE -Value `". \`"$completionFile\`"`"" -ForegroundColor Cyan
            Write-Host ""
        }
    }
    catch {
        # Silently ignore completion installation errors
    }
}

# Run installation
Install-CDM
