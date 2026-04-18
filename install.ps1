# gitlab-cli installer for Windows (PowerShell)
# Usage:
#   irm https://raw.githubusercontent.com/zhiyue/gitlab-cli/main/install.ps1 | iex
#   .\install.ps1                       # install latest
#   .\install.ps1 -Version v0.1.0      # specific version
#   .\install.ps1 -Dir 'C:\tools'      # specific dir
[CmdletBinding()]
param(
    [string]$Version = "",
    [string]$Dir    = "",
    [string]$Repo   = "zhiyue/gitlab-cli"
)

$ErrorActionPreference = "Stop"

$Target = "x86_64-pc-windows-msvc"
if (-not $Version) {
    $latest = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
    $Version = $latest.tag_name
}
if (-not $Dir) {
    $Dir = Join-Path $env:USERPROFILE ".local\bin"
}
if (-not (Test-Path $Dir)) {
    New-Item -ItemType Directory -Path $Dir -Force | Out-Null
}

$Archive  = "gitlab-cli-$Version-$Target.zip"
$SumName  = "gitlab-cli-$Version-$Target.zip.sha256"
$BaseUrl  = "https://github.com/$Repo/releases/download/$Version"
$Tmp      = New-Item -ItemType Directory -Path (Join-Path $env:TEMP "gitlab-cli-$([guid]::NewGuid())")

try {
    Write-Host "==> Downloading $Archive"
    Invoke-WebRequest -UseBasicParsing -Uri "$BaseUrl/$Archive" -OutFile (Join-Path $Tmp $Archive)
    try {
        Invoke-WebRequest -UseBasicParsing -Uri "$BaseUrl/$SumName" -OutFile (Join-Path $Tmp $SumName)
        $expected = (Get-Content (Join-Path $Tmp $SumName) -Raw).Trim().Split(" ")[0]
        $actual   = (Get-FileHash -Algorithm SHA256 (Join-Path $Tmp $Archive)).Hash.ToLower()
        if ($expected -ne $actual) { throw "checksum mismatch: expected $expected got $actual" }
        Write-Host "==> sha256 verified"
    } catch {
        Write-Warning "could not verify sha256 ($_)"
    }

    Expand-Archive -Path (Join-Path $Tmp $Archive) -DestinationPath $Tmp -Force
    Copy-Item -Path (Join-Path $Tmp "gitlab.exe") -Destination (Join-Path $Dir "gitlab.exe") -Force
    Write-Host "==> Installed: $Dir\gitlab.exe"
    & (Join-Path $Dir "gitlab.exe") --version

    if (-not ($env:Path -split ';' | Where-Object { $_ -eq $Dir })) {
        Write-Host ""
        Write-Host "Add $Dir to your PATH manually, or run:"
        Write-Host "  `$env:Path += ';$Dir'"
    }
    Write-Host ""
    Write-Host "Next: configure your token"
    Write-Host "  gitlab config set-token --host https://gitlab.example.com --token glpat-xxxxx"
} finally {
    Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}
