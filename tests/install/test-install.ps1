$ErrorActionPreference = "Stop"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$Repo = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$Installer = Join-Path $Repo "install.ps1"

$work = Join-Path $env:TEMP ("grove-ps1-test-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force -Path $work | Out-Null

$script:pass = 0
$script:fail = 0
function Report([bool]$Ok, [string]$Name) {
  if ($Ok) { $script:pass++; Write-Host "PASS: $Name" } else { $script:fail++; Write-Host "FAIL: $Name" }
}

$rsa = New-Object System.Security.Cryptography.RSACng 2048
$testParams = $rsa.ExportParameters($true)
$testModulusHex = ($testParams.Modulus | ForEach-Object { $_.ToString("X2") }) -join ""

function Bytes-Hex([byte[]]$Bytes) { ($Bytes | ForEach-Object { $_.ToString("X2") }) -join "" }

function ConvertTo-Base64Url([byte[]]$Bytes) {
  [Convert]::ToBase64String($Bytes) -replace '\+', '-' -replace '/', '_' -replace '=', ''
}

$server = Join-Path $work "server"
New-Item -ItemType Directory -Force -Path $server | Out-Null

$tarExe = Join-Path $env:SystemRoot "System32\tar.exe"
if (-not (Test-Path $tarExe)) { $tarExe = "tar.exe" }

$dtDir = Join-Path $work "dt"
New-Item -ItemType Directory -Force -Path (Join-Path $dtDir "ui\views") | Out-Null
[System.IO.File]::WriteAllText((Join-Path $dtDir "grove-desktop.exe"), "fake grove-desktop binary")
[System.IO.File]::WriteAllText((Join-Path $dtDir "ui\views\placeholder.hbs"), "placeholder")

foreach ($comp in @("grove", "grove-mcp")) {
  $fake = Join-Path $work "$comp.exe"
  [System.IO.File]::WriteAllText($fake, "fake $comp binary")
  & $tarExe -czf (Join-Path $server "$comp-v0.3.0-windows-x64.tar.gz") -C $work "$comp.exe"
  if ($LASTEXITCODE -ne 0) { throw "tar failed for $comp" }
}
& $tarExe -czf (Join-Path $server "grove-desktop-v0.3.0-windows-x64.tar.gz") -C $dtDir "grove-desktop.exe" "ui"
if ($LASTEXITCODE -ne 0) { throw "tar failed for grove-desktop" }

function New-Manifest([int]$Sequence, [int64]$ExpiresEpoch, [string]$Base) {
  $expiresIso = [DateTimeOffset]::FromUnixTimeSeconds($ExpiresEpoch).ToString("yyyy-MM-ddTHH:mm:ssZ")
  $entries = @()
  foreach ($comp in @("grove", "grove-mcp", "grove-desktop")) {
    $archive = Join-Path $server "$comp-v0.3.0-windows-x64.tar.gz"
    $sha = (Get-FileHash -Algorithm SHA256 $archive).Hash.ToLower()
    $size = (Get-Item $archive).Length
    $key = ($comp + "_windows_x64") -replace '-', '_'
    $entries += @"
        "$key": {
          "url": "$Base/v0.3.0/$comp-v0.3.0-windows-x64.tar.gz",
          "sha256": "$sha",
          "size": $size
        }
"@
  }
  $artifacts = $entries -join ",`n"
  $json = @"
{
  "version": "0.3.0",
  "created_at": "2026-08-05T00:00:00Z",
  "expires_at": "$expiresIso",
  "channels": {
    "stable": {
      "sequence": $Sequence,
      "artifacts": {
$artifacts
      }
    }
  }
}
"@
  $json = $json -replace "`r`n", "`n"
  [System.IO.File]::WriteAllText((Join-Path $server "manifest.json"), $json + "`n")
  $data = [System.IO.File]::ReadAllBytes((Join-Path $server "manifest.json"))
  $sha = [System.Security.Cryptography.SHA256]::Create().ComputeHash($data)
  $sig = $rsa.SignHash($sha, [System.Security.Cryptography.HashAlgorithmName]::SHA256, [System.Security.Cryptography.RSASignaturePadding]::Pss)
  [System.IO.File]::WriteAllText((Join-Path $server "manifest.json.sig"), (ConvertTo-Base64Url $sig) + "`n")
}

$script:counter = 0
function Run-Install([string[]]$ExtraArgs, [hashtable]$ExtraEnv = @{}) {
  $script:counter++
  $fakeHome = Join-Path $work "home$($script:counter)"
  $prefix = Join-Path $work "inst$($script:counter)"
  New-Item -ItemType Directory -Force -Path $fakeHome | Out-Null
  $envPairs = @{
    GROVE_FETCH_ROOT = $server
    GROVE_TRUSTED_MODULUS_HEX = $testModulusHex
    GROVE_HOME = (Join-Path $fakeHome ".grove")
  }
  foreach ($k in $ExtraEnv.Keys) { $envPairs[$k] = $ExtraEnv[$k] }
  $envFile = Join-Path $work "env$($script:counter).ps1"
  $lines = @()
  foreach ($k in $envPairs.Keys) { $lines += "`$env:$k = '$($envPairs[$k] -replace "'", "''")'" }
  $argStr = ((@("-Prefix", $prefix) + $ExtraArgs) | ForEach-Object { if ($_ -match '^-') { $_ } else { "'$_'" } }) -join " "
  $lines += "& '$Installer' $argStr"
  $lines += "exit `$LASTEXITCODE"
  [System.IO.File]::WriteAllText($envFile, ($lines -join "`n"))
  $out = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $envFile 2>&1 | Out-String
  return @{ Rc = $LASTEXITCODE; Out = $out; Home = $fakeHome; Prefix = $prefix }
}

$goodBase = "https://github.com/alxshelepenok/grove/releases/download"
$now = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()

New-Manifest 7 ($now + 86400) $goodBase
$r = Run-Install @()
Report ($r.Rc -eq 0) "happy path installs"
Report ($r.Out -match "WARNING: GROVE_FETCH_ROOT is set") "trust-override hooks print a loud warning"
Report ((Test-Path (Join-Path $r.Prefix "bin\grove.exe")) -and (Test-Path (Join-Path $r.Prefix "bin\grove-mcp.exe"))) "binaries installed"
Report ((Test-Path (Join-Path $r.Prefix "grove-desktop\grove-desktop.exe")) -and (Test-Path (Join-Path $r.Prefix "grove-desktop\ui\views\placeholder.hbs"))) "desktop app installed with ui templates"
Report ([System.IO.File]::ReadAllText((Join-Path $r.Home ".grove\.sequence")) -match "stable=7") "sequence file written"

$r = Run-Install @("-Only", "grove-mcp")
Report (($r.Rc -eq 0) -and (Test-Path (Join-Path $r.Prefix "bin\grove-mcp.exe")) -and (-not (Test-Path (Join-Path $r.Prefix "bin\grove.exe"))) -and (-not (Test-Path (Join-Path $r.Prefix "grove-desktop")))) "-Only grove-mcp installs just one component"

$r = Run-Install @("-Only", "grove-desktop")
Report (($r.Rc -eq 0) -and (Test-Path (Join-Path $r.Prefix "grove-desktop\grove-desktop.exe")) -and (-not (Test-Path (Join-Path $r.Prefix "bin")))) "-Only grove-desktop installs just the desktop app"

New-Manifest 7 ($now + 86400) $goodBase
Add-Content (Join-Path $server "manifest.json") "tampered"
$r = Run-Install @()
Report (($r.Rc -ne 0) -and ($r.Out -match "signature verification failed")) "tampered manifest rejected"

New-Manifest 7 ($now - 90000) $goodBase
$r = Run-Install @()
Report (($r.Rc -ne 0) -and ($r.Out -match "Manifest expired")) "expired manifest rejected"

New-Manifest 1 ($now + 86400) $goodBase
$seqHome = Join-Path $work "home$($script:counter + 1)"
New-Item -ItemType Directory -Force -Path (Join-Path $seqHome ".grove") | Out-Null
[System.IO.File]::WriteAllText((Join-Path $seqHome ".grove\.sequence"), "format 1`nstable=9`n")
$r = Run-Install @()
Report (($r.Rc -ne 0) -and ($r.Out -match "rollback")) "rolled-back sequence rejected"

New-Manifest 7 ($now + 86400) "https://evil.example.com/dl"
$r = Run-Install @()
Report (($r.Rc -ne 0) -and ($r.Out -match "allowed host")) "wrong-host artifact url rejected"

New-Manifest 7 ($now + 86400) $goodBase
Add-Content (Join-Path $server "grove-v0.3.0-windows-x64.tar.gz") "corrupted"
$r = Run-Install @()
Report (($r.Rc -ne 0) -and ($r.Out -match "mismatch")) "hash/size mismatch rejected"

$r = Run-Install @() @{ GROVE_ARTIFACT_URL = "$goodBase/v0.3.0/grove-mcp-v0.3.0-windows-x64.tar.gz" }
Report (($r.Rc -eq 0) -and (Test-Path (Join-Path $r.Prefix "bin\grove-mcp.exe"))) "break-glass install works"
Report ($r.Out -match "WARNING: GROVE_ARTIFACT_URL break-glass") "break-glass prints a loud warning"
Report (-not (Test-Path (Join-Path $r.Home ".grove\.sequence"))) "break-glass does not touch anti-rollback state"

if (Get-Command bash -ErrorAction SilentlyContinue) {
  New-Manifest 7 ($now + 86400) $goodBase
  $repoPosix = ($Repo -replace '\\', '/')
  $serverPosix = ($server -replace '\\', '/')
  $osslModulus = & bash -lc "openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out '$serverPosix/ossl-key.pem' 2>/dev/null && '$repoPosix/bin/sign.sh' '$serverPosix/ossl-key.pem' '$serverPosix/manifest.json' '$serverPosix/manifest.json.sig' && openssl rsa -in '$serverPosix/ossl-key.pem' -pubout -modulus -noout 2>/dev/null | cut -d= -f2 | tr -d '[:space:]'" | Out-String
  $osslModulus = $osslModulus.Trim()
  if ($osslModulus -match '^[0-9A-F]{512}$') {
    $r = Run-Install @() @{ GROVE_TRUSTED_MODULUS_HEX = $osslModulus }
    Report ($r.Rc -eq 0) "openssl-signed manifest verifies under RSACng (production pairing)"
  } else {
    Report $false "openssl-signed manifest verifies under RSACng (production pairing)"
  }
  Remove-Item (Join-Path $server "ossl-key.pem") -ErrorAction SilentlyContinue
}

Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
Write-Host "$($script:pass) passed, $($script:fail) failed"
if ($script:fail -eq 0) { exit 0 } else { exit 1 }
