# Runs the UI test suite inside Windows Sandbox so the host desktop is never
# grabbed. Skips with a clear message when the Sandbox feature is unavailable.
#
#   scripts/sandbox/run.ps1

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

function Test-Sandbox {
    try {
        (Get-WindowsOptionalFeature -Online -FeatureName 'Containers-DisposableClientVM').State -eq 'Enabled'
    } catch { $false }
}

if (-not (Test-Sandbox)) {
    Write-Output 'SKIP Windows Sandbox is not enabled; run UI tests on a spare desktop instead.'
    exit 0
}

$map = Join-Path $env:TEMP "visualrust-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $map | Out-Null
$cmd = Join-Path $map 'run.ps1'
@"
Set-Location C:\visualrust
cargo test --workspace
"@ | Set-Content -Path $cmd -Encoding UTF8

$wsb = Join-Path $env:TEMP "visualrust-$([guid]::NewGuid().ToString('N')).wsb"
@"
<Configuration>
  <MappedFolders>
    <MappedFolder><HostFolder>$repo</HostFolder><SandboxFolder>C:\visualrust</SandboxFolder><ReadOnly>false</ReadOnly></MappedFolder>
  </MappedFolders>
  <LogonCommand><Command>powershell.exe -ExecutionPolicy Bypass -File C:\visualrust\scripts\sandbox\run.ps1</Command></LogonCommand>
</Configuration>
"@ | Set-Content -Path $wsb -Encoding UTF8

Write-Output "Launching Windows Sandbox: $wsb"
Start-Process -FilePath 'WindowsSandbox.exe' -ArgumentList $wsb -Wait
