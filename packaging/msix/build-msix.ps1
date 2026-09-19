<#
.SYNOPSIS
  Packages the built Cleanup Assist executable as an MSIX for the Microsoft Store.

.DESCRIPTION
  Tauri does not emit MSIX, so this assembles one from the release executable plus
  the Square*/StoreLogo assets Tauri's icon generator already produces, then packs
  it with makeappx.exe from the Windows SDK.

  The resulting package is UNSIGNED, which is correct for Store submission:
  Microsoft re-signs MSIX packages after certification, so no certificate is
  needed. An unsigned MSIX cannot be side-loaded for local testing - sign it with
  a self-signed certificate and trust that certificate if you want to install it
  yourself.

.PARAMETER Version
  Three-part app version, e.g. 0.2.4. Expanded to the four-part form MSIX requires.

.PARAMETER IdentityName
  Partner Center's "Package/Identity/Name". Must match exactly or upload is rejected.

.PARAMETER Publisher
  Partner Center's "Package/Identity/Publisher", e.g. CN=ABCD1234-...

.PARAMETER PublisherDisplayName
  Partner Center's "Package/Properties/PublisherDisplayName".

.EXAMPLE
  ./packaging/msix/build-msix.ps1 -Version 0.2.4
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][string]$Version,
  [string]$IdentityName = $env:MSIX_IDENTITY_NAME,
  [string]$Publisher = $env:MSIX_PUBLISHER,
  [string]$PublisherDisplayName = $env:MSIX_PUBLISHER_DISPLAY_NAME,
  [string]$ExePath = "target/release/cleanup-assist.exe",
  [string]$OutFile = ""
)

$ErrorActionPreference = "Stop"

# Placeholder identity, used until Partner Center values are supplied. A package
# built with these is structurally valid and good for testing the pipeline, but
# will be rejected at upload.
if (-not $IdentityName)         { $IdentityName = "RichardGaskin.CleanupAssist" }
if (-not $Publisher)            { $Publisher = "CN=00000000-0000-0000-0000-000000000000" }
if (-not $PublisherDisplayName) { $PublisherDisplayName = "Richard Gaskin" }

$usingPlaceholder = $Publisher -like "CN=00000000-*"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "../..")
Push-Location $repoRoot
try {
  # --- version: MSIX needs Major.Minor.Build.Revision, revision 0 for the Store
  if ($Version -notmatch '^\d+\.\d+\.\d+$') {
    throw "Version must look like 1.2.3, got '$Version'"
  }
  $msixVersion = "$Version.0"

  if (-not (Test-Path $ExePath)) { throw "executable not found: $ExePath" }
  if (-not $OutFile) { $OutFile = "target/release/bundle/msix/CleanupAssist_${Version}_x64.msix" }

  # --- stage the package payload
  $stage = Join-Path ([System.IO.Path]::GetTempPath()) "cleanup-assist-msix-$Version"
  if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
  New-Item -ItemType Directory -Force -Path $stage, (Join-Path $stage "Assets") | Out-Null

  Copy-Item $ExePath (Join-Path $stage "cleanup-assist.exe")

  # Tauri's icon generator already emits these under the exact MSIX asset names.
  $assetNames = @(
    "StoreLogo.png", "Square44x44Logo.png", "Square71x71Logo.png",
    "Square150x150Logo.png"
  )
  foreach ($name in $assetNames) {
    $src = Join-Path $repoRoot "src-tauri/icons/$name"
    if (-not (Test-Path $src)) { throw "missing icon asset: $src" }
    Copy-Item $src (Join-Path $stage "Assets/$name")
  }

  # --- render the manifest
  $template = Get-Content (Join-Path $PSScriptRoot "AppxManifest.template.xml") -Raw -Encoding UTF8
  $manifest = $template.
    Replace("{{IDENTITY_NAME}}", $IdentityName).
    Replace("{{PUBLISHER}}", $Publisher).
    Replace("{{PUBLISHER_DISPLAY_NAME}}", $PublisherDisplayName).
    Replace("{{VERSION}}", $msixVersion)
  $utf8NoBom = New-Object System.Text.UTF8Encoding $false
  [System.IO.File]::WriteAllText((Join-Path $stage "AppxManifest.xml"), $manifest, $utf8NoBom)

  # --- locate makeappx.exe from the Windows SDK (newest version wins)
  $makeappx = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\makeappx.exe" -ErrorAction SilentlyContinue |
    Sort-Object { [version]($_.Directory.Parent.Name) } -Descending |
    Select-Object -First 1 -ExpandProperty FullName
  if (-not $makeappx) {
    throw "makeappx.exe not found. Install the Windows 10/11 SDK."
  }

  New-Item -ItemType Directory -Force -Path (Split-Path $OutFile) | Out-Null
  if (Test-Path $OutFile) { Remove-Item $OutFile -Force }

  Write-Host "Packing MSIX $msixVersion"
  Write-Host "  identity  $IdentityName"
  Write-Host "  publisher $Publisher"
  & $makeappx pack /d $stage /p $OutFile /o
  if ($LASTEXITCODE -ne 0) { throw "makeappx failed with exit code $LASTEXITCODE" }

  Remove-Item $stage -Recurse -Force
  $size = [math]::Round((Get-Item $OutFile).Length / 1MB, 2)
  Write-Host "Built $OutFile ($size MB)"

  if ($usingPlaceholder) {
    Write-Warning "Built with PLACEHOLDER identity. Partner Center will reject this package."
    Write-Warning "Set MSIX_IDENTITY_NAME / MSIX_PUBLISHER / MSIX_PUBLISHER_DISPLAY_NAME first."
  }
}
finally {
  Pop-Location
}
