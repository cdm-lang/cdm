# CDM CLI Uninstaller for Windows PowerShell
# Usage: irm https://raw.githubusercontent.com/cdm-lang/cdm/main/uninstall.ps1 | iex

$ErrorActionPreference = 'Stop'

$InstallDir = if ($env:CDM_INSTALL_DIR) { $env:CDM_INSTALL_DIR } else { "$env:LOCALAPPDATA\cdm" }
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
}

# Remove from PATH
function Remove-FromPath {
    param([string]$Directory)

    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")

    if ($userPath -like "*$Directory*") {
        Write-Info "Removing $Directory from PATH..."

        # Split path, remove the directory, and rejoin
        $pathArray = $userPath -split ';' | Where-Object { $_ -ne $Directory -and $_ -ne '' }
        $newPath = $pathArray -join ';'

        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")

        # Update current session PATH
        $env:Path = ($env:Path -split ';' | Where-Object { $_ -ne $Directory -and $_ -ne '' }) -join ';'

        Write-Info "Removed from PATH successfully"
        return $true
    }

    return $false
}

# Remove shell completions
function Remove-Completions {
    $profileDir = Split-Path -Parent $PROFILE
    $completionFile = Join-Path $profileDir "cdm-completion.ps1"

    if (Test-Path $completionFile) {
        Remove-Item $completionFile -Force
        Write-Info "Removed PowerShell completions from $completionFile"
        Write-Host ""
        Write-Warn "You may want to remove this line from your PowerShell profile ($PROFILE):"
        Write-Host "    . `"$completionFile`"" -ForegroundColor Yellow
        Write-Host ""
    }
    else {
        Write-Info "No PowerShell completions found"
    }
}

# Remove plugin cache
function Remove-Cache {
    $cacheDir = Join-Path $env:LOCALAPPDATA "cdm"

    if (Test-Path $cacheDir) {
        Remove-Item -Recurse -Force $cacheDir
        Write-Info "Removed plugin cache from $cacheDir"
    }
    else {
        Write-Info "No plugin cache found"
    }
}

# Main uninstall function
function Uninstall-CDM {
    Write-Info "Uninstalling CDM CLI..."
    Write-Host ""

    # Check if CDM is installed
    $installPath = Join-Path $InstallDir "bin\$BinaryName"

    if (-not (Test-Path $InstallDir) -and -not (Test-Path $installPath)) {
        Write-Warn "CDM CLI does not appear to be installed at $InstallDir"
        Write-Host ""
        Write-Info "Checking for completions and cache anyway..."
        Remove-Completions
        Write-Host ""
        Remove-Cache
        return
    }

    # Remove the installation directory
    if (Test-Path $InstallDir) {
        Remove-Item -Recurse -Force $InstallDir
        Write-Info "Removed CDM CLI from $InstallDir"
    }

    # Remove shell completions
    Write-Host ""
    Remove-Completions

    # Remove plugin cache
    Write-Host ""
    Remove-Cache

    # Remove from PATH
    Write-Host ""
    $removedFromPath = Remove-FromPath -Directory "$InstallDir\bin"

    Write-Host ""
    Write-Info "CDM CLI has been uninstalled successfully!"

    if ($removedFromPath) {
        Write-Host ""
        Write-Warn "You may need to restart your terminal for PATH changes to take effect"
    }
}

# Run uninstallation
Uninstall-CDM
