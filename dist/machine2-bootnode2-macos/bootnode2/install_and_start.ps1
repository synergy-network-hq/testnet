$ErrorActionPreference = "Stop"

$BaseDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$EnvPath = Join-Path $BaseDir "node.env"
$NodeEnv = @{}
Get-Content $EnvPath | ForEach-Object {
  if ($_ -match '^\s*$' -or $_ -match '^\s*#') { return }
  $parts = $_ -split '=', 2
  if ($parts.Count -eq 2) { $NodeEnv[$parts[0].Trim()] = $parts[1].Trim() }
}

$BinPath = Join-Path $BaseDir "bin/synergy-testbeta-windows-amd64.exe"
$ConfigPath = Join-Path $BaseDir "config/node.toml"
$GenesisPath = Join-Path $BaseDir "config/genesis.json"
$DataDir = Join-Path $BaseDir "data"
$LogsDir = Join-Path $DataDir "logs"
$ChainDir = Join-Path $DataDir "chain"
$ChainStateFile = Join-Path $DataDir "chain.json"
$PidFile = Join-Path $DataDir "node.pid"
$OutFile = Join-Path $LogsDir "node.out"
$ErrFile = Join-Path $LogsDir "node.err"

New-Item -ItemType Directory -Force -Path $LogsDir | Out-Null
if (-not (Test-Path $GenesisPath)) { throw "Missing canonical genesis file: $GenesisPath" }
if (Test-Path $ChainDir) { Remove-Item -Recurse -Force $ChainDir }
if (Test-Path $ChainStateFile) { Remove-Item -Force $ChainStateFile }
New-Item -ItemType Directory -Force -Path $ChainDir | Out-Null

if (-not (Test-Path $BinPath)) { throw "Missing Windows binary: $BinPath" }

$previousProjectRoot = $env:SYNERGY_PROJECT_ROOT
$previousConfigPath = $env:SYNERGY_CONFIG_PATH
$previousGenesisPath = $env:SYNERGY_GENESIS_FILE
$previousBootstrapOnly = $env:SYNERGY_BOOTSTRAP_ONLY
$previousAutoRegister = $env:SYNERGY_AUTO_REGISTER_VALIDATOR
$previousP2PListenAddress = $env:SYNERGY_P2P_LISTEN_ADDRESS
$previousP2PExternalAddress = $env:SYNERGY_P2P_EXTERNAL_ADDRESS
$previousP2PPublicAddress = $env:SYNERGY_P2P_PUBLIC_ADDRESS
$previousDiscoveryPort = $env:SYNERGY_DISCOVERY_PORT
$previousDiscoveryListenAddress = $env:SYNERGY_DISCOVERY_LISTEN_ADDRESS
$previousDiscoveryExternalAddress = $env:SYNERGY_DISCOVERY_EXTERNAL_ADDRESS
$previousDiscoveryPublicAddress = $env:SYNERGY_DISCOVERY_PUBLIC_ADDRESS

$env:SYNERGY_PROJECT_ROOT = $BaseDir
$env:SYNERGY_CONFIG_PATH = $ConfigPath
$env:SYNERGY_GENESIS_FILE = $GenesisPath
$env:SYNERGY_BOOTSTRAP_ONLY = "true"
$env:SYNERGY_AUTO_REGISTER_VALIDATOR = "false"
if ($NodeEnv.ContainsKey("P2P_LISTEN_ADDRESS")) { $env:SYNERGY_P2P_LISTEN_ADDRESS = $NodeEnv["P2P_LISTEN_ADDRESS"] }
if ($NodeEnv.ContainsKey("P2P_EXTERNAL_ADDRESS")) { $env:SYNERGY_P2P_EXTERNAL_ADDRESS = $NodeEnv["P2P_EXTERNAL_ADDRESS"] }
if ($NodeEnv.ContainsKey("P2P_PUBLIC_ADDRESS")) { $env:SYNERGY_P2P_PUBLIC_ADDRESS = $NodeEnv["P2P_PUBLIC_ADDRESS"] }
if ($NodeEnv.ContainsKey("DISCOVERY_PORT")) { $env:SYNERGY_DISCOVERY_PORT = $NodeEnv["DISCOVERY_PORT"] }
if ($NodeEnv.ContainsKey("DISCOVERY_LISTEN_ADDRESS")) { $env:SYNERGY_DISCOVERY_LISTEN_ADDRESS = $NodeEnv["DISCOVERY_LISTEN_ADDRESS"] }
if ($NodeEnv.ContainsKey("DISCOVERY_EXTERNAL_ADDRESS")) { $env:SYNERGY_DISCOVERY_EXTERNAL_ADDRESS = $NodeEnv["DISCOVERY_EXTERNAL_ADDRESS"] }
if ($NodeEnv.ContainsKey("DISCOVERY_PUBLIC_ADDRESS")) { $env:SYNERGY_DISCOVERY_PUBLIC_ADDRESS = $NodeEnv["DISCOVERY_PUBLIC_ADDRESS"] }

try {
  $proc = Start-Process -FilePath $BinPath -ArgumentList @("start", "--config", $ConfigPath) -WorkingDirectory $BaseDir -RedirectStandardOutput $OutFile -RedirectStandardError $ErrFile -PassThru -WindowStyle Hidden
}
finally {
  $env:SYNERGY_PROJECT_ROOT = $previousProjectRoot
  $env:SYNERGY_CONFIG_PATH = $previousConfigPath
  $env:SYNERGY_GENESIS_FILE = $previousGenesisPath
  $env:SYNERGY_BOOTSTRAP_ONLY = $previousBootstrapOnly
  $env:SYNERGY_AUTO_REGISTER_VALIDATOR = $previousAutoRegister
  $env:SYNERGY_P2P_LISTEN_ADDRESS = $previousP2PListenAddress
  $env:SYNERGY_P2P_EXTERNAL_ADDRESS = $previousP2PExternalAddress
  $env:SYNERGY_P2P_PUBLIC_ADDRESS = $previousP2PPublicAddress
  $env:SYNERGY_DISCOVERY_PORT = $previousDiscoveryPort
  $env:SYNERGY_DISCOVERY_LISTEN_ADDRESS = $previousDiscoveryListenAddress
  $env:SYNERGY_DISCOVERY_EXTERNAL_ADDRESS = $previousDiscoveryExternalAddress
  $env:SYNERGY_DISCOVERY_PUBLIC_ADDRESS = $previousDiscoveryPublicAddress
}

$proc.Id | Set-Content -Path $PidFile
"Started $($NodeEnv["MACHINE_ID"]) as bootstrap-only discovery node (PID $($proc.Id))"
