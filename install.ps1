param(
  [string]$Channel = "stable",
  [string]$Version = "",
  [string[]]$Only = @("grove", "grove-mcp", "grove-desktop"),
  [string]$Prefix = "$HOME\.local\grove"
)

$ErrorActionPreference = "Stop"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

foreach ($hook in @("GROVE_TRUSTED_MODULUS_HEX", "GROVE_FETCH_ROOT", "GROVE_HOME")) {
  if (Get-Item "env:$hook" -ErrorAction SilentlyContinue) {
    Write-Host "WARNING: $hook is set - trust/store override active (test hook, not for production use)" -ForegroundColor Yellow
  }
}

$GroveRepo = "alxshelepenok/grove"
$RawBase = "https://raw.githubusercontent.com/$GroveRepo/main"
$ReleaseBase = "https://github.com/$GroveRepo/releases/download"
$MinimumSequence = 1

$TrustedModulusHex = "ABBF5371AB06F0070268659A50EE04966123C267CBF13364E5DC999C28960590CCF579057BCDF497A56E5683E4E6489F0708C1F87026EC86705D103D96F30315379ABC9A73A3A352C86A41E13B2EE6DD6992F62595287DA23E4787E0A237CD16137AB2FB87B80EF9E91A29EAA632645258F68E754B4EAA3964247A6F57174BA3D2B57238EC7370C4B08EE7FDD2EE42557779DC523C7282B7E8CF14C55814E2F66602F7376DFE47FED93EF191D3D6F91AA1710DECC59A2090DB1A21453E09E3EDDF8F62D725899416DE7A3A4F69D244F3F1178575A2CD3713A8A87CF93AE5051ABC54442E2F41B9DEE7E9389112ED63E873AC68E841B75EB4C6D251C601908E15"
$TrustedExponent = [byte[]](1, 0, 1)

function Die([string]$Message) {
  Write-Host "error: $Message" -ForegroundColor Red
  exit 1
}

function HexToBytes([string]$Hex) {
  $bytes = New-Object byte[] ($Hex.Length / 2)
  for ($i = 0; $i -lt $bytes.Length; $i++) {
    $bytes[$i] = [Convert]::ToByte($Hex.Substring($i * 2, 2), 16)
  }
  return ,$bytes
}

function Fetch([string]$Url, [string]$OutFile) {
  if ($env:GROVE_FETCH_ROOT) {
    $name = Split-Path $Url -Leaf
    $src = Join-Path $env:GROVE_FETCH_ROOT $name
    if (-not (Test-Path $src)) { Die "fetch failed (test root): $Url" }
    Copy-Item $src $OutFile
    return
  }
  try {
    Invoke-WebRequest -Uri $Url -OutFile $OutFile -UseBasicParsing | Out-Null
  } catch {
    Die "could not download $Url - check your internet connection and try again"
  }
}

function ConvertFrom-Base64Url([string]$Text) {
  $b64 = ($Text -replace '\s', '') -replace '-', '+' -replace '_', '/'
  switch ($b64.Length % 4) {
    2 { $b64 += "==" }
    3 { $b64 += "=" }
    1 { Die "malformed base64url signature" }
  }
  return ,[Convert]::FromBase64String($b64)
}

function Test-ManifestSignature([string]$File, [string]$SigFile) {
  $data = [System.IO.File]::ReadAllBytes($File)
  $sig = ConvertFrom-Base64Url ([System.IO.File]::ReadAllText($SigFile))
  $sha = [System.Security.Cryptography.SHA256]::Create().ComputeHash($data)
  if ($env:GROVE_TRUSTED_MODULUS_HEX) {
    $modulus = HexToBytes $env:GROVE_TRUSTED_MODULUS_HEX
  } else {
    $modulus = HexToBytes $TrustedModulusHex
  }
  $params = New-Object System.Security.Cryptography.RSAParameters
  $params.Modulus = $modulus
  $params.Exponent = $TrustedExponent
  $rsa = New-Object System.Security.Cryptography.RSACng
  $rsa.ImportParameters($params)
  return $rsa.VerifyHash($sha, $sig, [System.Security.Cryptography.HashAlgorithmName]::SHA256, [System.Security.Cryptography.RSASignaturePadding]::Pss)
}

function Get-SequenceFile {
  if ($env:GROVE_HOME) { return Join-Path $env:GROVE_HOME ".sequence" }
  return Join-Path $HOME ".grove\.sequence"
}

function Read-Sequence([string]$Chan) {
  $seqFile = Get-SequenceFile
  if (-not (Test-Path $seqFile)) { return $null }
  foreach ($line in [System.IO.File]::ReadAllLines($seqFile)) {
    if ($line -match "^$Chan=([0-9]+)\r?$") { return [int]$Matches[1] }
  }
  return $null
}

function Write-Sequence([string]$Chan, [int]$Seq) {
  $seqFile = Get-SequenceFile
  $seqDir = Split-Path $seqFile
  New-Item -ItemType Directory -Force -Path $seqDir | Out-Null
  $lines = @()
  if (Test-Path $seqFile) {
    $lines = @([System.IO.File]::ReadAllLines($seqFile) | Where-Object { $_ -notmatch "^$Chan=" })
  } else {
    $lines = @("format 1")
  }
  $lines += "$Chan=$Seq"
  $tmp = Join-Path $seqDir (".sequence.tmp." + [Guid]::NewGuid().ToString("N"))
  [System.IO.File]::WriteAllText($tmp, ($lines -join "`n") + "`n")
  Move-Item -Force $tmp $seqFile
}

function Install-Archive([string]$Archive, [string]$Comp, [string]$Mver) {
  $unpack = Join-Path $env:TEMP ("grove-install-" + [Guid]::NewGuid().ToString("N"))
  New-Item -ItemType Directory -Force -Path $unpack | Out-Null
  $tarExe = Join-Path $env:SystemRoot "System32\tar.exe"
  if (-not (Test-Path $tarExe)) { $tarExe = "tar.exe" }
  try {
    & $tarExe -xzf $Archive -C $unpack
    if ($LASTEXITCODE -ne 0) { Die "failed to unpack $Archive (tar.exe is required, present on Windows 10+)" }
    $binname = "$Comp.exe"
    $binpath = Get-ChildItem -Recurse -File -Filter $binname $unpack | Select-Object -First 1
    if (-not $binpath) { Die "archive does not contain $binname" }
    if ($Comp -eq "grove-desktop") {
      $dest = Join-Path $Prefix "grove-desktop"
      if (Test-Path $dest) { Remove-Item -Recurse -Force $dest }
      New-Item -ItemType Directory -Force -Path $dest | Out-Null
      Copy-Item -Recurse (Join-Path $unpack "*") $dest
      if (-not (Test-Path (Join-Path $dest "ui\views"))) { Die "desktop archive is missing ui/views" }
      Write-Host "installed grove-desktop ($Mver) to $dest"
      return
    }
    $bindir = Join-Path $Prefix "bin"
    New-Item -ItemType Directory -Force -Path $bindir | Out-Null
    Copy-Item $binpath.FullName (Join-Path $bindir $binname) -Force
    Write-Host "installed $binname ($Mver) to $bindir\$binname"
  } finally {
    Remove-Item -Recurse -Force $unpack -ErrorAction SilentlyContinue
  }
}

function Invoke-BreakGlass {
  $url = $env:GROVE_ARTIFACT_URL
  Write-Host "WARNING: GROVE_ARTIFACT_URL break-glass mode - skipping manifest, signature, and hash verification" -ForegroundColor Yellow
  Write-Host "WARNING: trust is delegated entirely to you and the channel that delivered this URL" -ForegroundColor Yellow
  $archive = Join-Path $env:TEMP "grove-breakglass.tar.gz"
  Fetch $url $archive
  $comp = "grove"
  if ((Split-Path $url -Leaf) -like "grove-mcp-*") { $comp = "grove-mcp" }
  if ((Split-Path $url -Leaf) -like "grove-desktop-*") { $comp = "grove-desktop" }
  Install-Archive $archive $comp "break-glass"
  Write-Host "break-glass install complete; anti-rollback state was not updated"
  exit 0
}

function Invoke-MainInstall {
  if ($Version) {
    $manifestUrl = "$ReleaseBase/v$Version/manifest.json"
  } else {
    $manifestUrl = "$RawBase/manifest.json"
  }

  $tmp = Join-Path $env:TEMP ("grove-manifest-" + [Guid]::NewGuid().ToString("N"))
  New-Item -ItemType Directory -Force -Path $tmp | Out-Null
  $manifestFile = Join-Path $tmp "manifest.json"
  $sigFile = Join-Path $tmp "manifest.json.sig"
  Fetch $manifestUrl $manifestFile
  Fetch "$manifestUrl.sig" $sigFile

  if (-not (Test-ManifestSignature $manifestFile $sigFile)) {
    Die "manifest signature verification failed - refusing to parse or install"
  }

  $manifest = Get-Content $manifestFile -Raw | ConvertFrom-Json
  $mVersion = $manifest.version
  $mExpires = $manifest.expires_at
  $channelData = $manifest.channels.$Channel
  if (-not $channelData) { Die "manifest has no channel $Channel" }
  $mSequence = [int]$channelData.sequence
  if ($Version -and ($mVersion -ne $Version)) {
    Die "manifest version $mVersion does not match requested $Version"
  }

  $now = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
  $expiresEpoch = [DateTimeOffset]::Parse($mExpires).ToUnixTimeSeconds()
  if ($now -gt ($expiresEpoch + 86400)) {
    Die "Manifest expired. A new release is pending; try again later."
  }

  $stored = Read-Sequence $Channel
  if (($null -ne $stored) -and ($mSequence -le $stored)) {
    Die "manifest sequence $mSequence is not newer than installed sequence $stored - possible rollback, refusing"
  }
  if ($mSequence -lt $MinimumSequence) {
    Die "manifest sequence $mSequence is below the minimum $MinimumSequence"
  }

  foreach ($comp in $Only) {
    $key = ($comp + "_windows_x64") -replace '-', '_'
    $artifact = $channelData.artifacts.$key
    if (-not $artifact) { Die "manifest has no artifact for $key" }
    if (-not $artifact.url.StartsWith("$ReleaseBase/v$mVersion/")) {
      Die "artifact URL for $key is not on the allowed host: $($artifact.url)"
    }
    $archive = Join-Path $tmp "$comp.tar.gz"
    Fetch $artifact.url $archive
    $actualSize = (Get-Item $archive).Length
    if ($actualSize -ne [int64]$artifact.size) {
      Die "size mismatch for ${key}: expected $($artifact.size), got $actualSize"
    }
    $actualSha = (Get-FileHash -Algorithm SHA256 $archive).Hash.ToLower()
    if ($actualSha -ne $artifact.sha256.ToLower()) {
      Die "sha256 mismatch for $key - the downloaded bytes do not match the signed manifest"
    }
    Install-Archive $archive $comp $mVersion
  }

  Write-Sequence $Channel $mSequence
  Write-Host "grove $mVersion installed (channel $Channel, sequence $mSequence)"
  Write-Host "add $Prefix\bin to your PATH if it is not already there"
}

if ($env:GROVE_ARTIFACT_URL) {
  Invoke-BreakGlass
}

Invoke-MainInstall
