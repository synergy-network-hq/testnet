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
$previousP2PPort = $env:SYNERGY_P2P_PORT
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
$bindIp = if ($NodeEnv.ContainsKey("BIND_IP")) { $NodeEnv["BIND_IP"] } else { "0.0.0.0" }
$publicHost = if ($NodeEnv.ContainsKey("NODE_PUBLIC_HOST")) { $NodeEnv["NODE_PUBLIC_HOST"] } elseif ($NodeEnv.ContainsKey("HOSTNAME")) { $NodeEnv["HOSTNAME"] } elseif ($NodeEnv.ContainsKey("HOST")) { $NodeEnv["HOST"] } else { "" }
$p2pPort = if ($NodeEnv.ContainsKey("P2P_PORT")) { $NodeEnv["P2P_PORT"] } else { "" }
$publicP2PPort = if ($NodeEnv.ContainsKey("PUBLIC_P2P_PORT")) { $NodeEnv["PUBLIC_P2P_PORT"] } elseif ($NodeEnv.ContainsKey("P2P_PORT_EXTERNAL")) { $NodeEnv["P2P_PORT_EXTERNAL"] } else { $p2pPort }
if (-not [string]::IsNullOrWhiteSpace($p2pPort)) { $env:SYNERGY_P2P_PORT = $p2pPort }
if (-not [string]::IsNullOrWhiteSpace($p2pPort)) {
  $env:SYNERGY_P2P_LISTEN_ADDRESS = "${bindIp}:$p2pPort"
} elseif ($NodeEnv.ContainsKey("P2P_LISTEN_ADDRESS")) {
  $env:SYNERGY_P2P_LISTEN_ADDRESS = $NodeEnv["P2P_LISTEN_ADDRESS"]
}
if (-not [string]::IsNullOrWhiteSpace($publicHost) -and -not [string]::IsNullOrWhiteSpace($publicP2PPort)) {
  $env:SYNERGY_P2P_EXTERNAL_ADDRESS = "${publicHost}:$publicP2PPort"
  $env:SYNERGY_P2P_PUBLIC_ADDRESS = "${publicHost}:$publicP2PPort"
} elseif ($NodeEnv.ContainsKey("P2P_EXTERNAL_ADDRESS")) {
  $env:SYNERGY_P2P_EXTERNAL_ADDRESS = $NodeEnv["P2P_EXTERNAL_ADDRESS"]
  $env:SYNERGY_P2P_PUBLIC_ADDRESS = $NodeEnv["P2P_EXTERNAL_ADDRESS"]
} elseif ($NodeEnv.ContainsKey("P2P_PUBLIC_ADDRESS")) {
  $env:SYNERGY_P2P_EXTERNAL_ADDRESS = $NodeEnv["P2P_PUBLIC_ADDRESS"]
  $env:SYNERGY_P2P_PUBLIC_ADDRESS = $NodeEnv["P2P_PUBLIC_ADDRESS"]
}
$discoveryPort = if ($NodeEnv.ContainsKey("DISCOVERY_PORT")) { $NodeEnv["DISCOVERY_PORT"] } else { "" }
$discoveryPublicPort = if ($NodeEnv.ContainsKey("DISCOVERY_PORT_EXTERNAL")) { $NodeEnv["DISCOVERY_PORT_EXTERNAL"] } else { $discoveryPort }
if (-not [string]::IsNullOrWhiteSpace($discoveryPort)) { $env:SYNERGY_DISCOVERY_PORT = $discoveryPort }
if (-not [string]::IsNullOrWhiteSpace($discoveryPort)) {
  $env:SYNERGY_DISCOVERY_LISTEN_ADDRESS = "${bindIp}:$discoveryPort"
} elseif ($NodeEnv.ContainsKey("DISCOVERY_LISTEN_ADDRESS")) {
  $env:SYNERGY_DISCOVERY_LISTEN_ADDRESS = $NodeEnv["DISCOVERY_LISTEN_ADDRESS"]
}
if (-not [string]::IsNullOrWhiteSpace($publicHost) -and -not [string]::IsNullOrWhiteSpace($discoveryPublicPort)) {
  $env:SYNERGY_DISCOVERY_EXTERNAL_ADDRESS = "${publicHost}:$discoveryPublicPort"
  $env:SYNERGY_DISCOVERY_PUBLIC_ADDRESS = "${publicHost}:$discoveryPublicPort"
} elseif ($NodeEnv.ContainsKey("DISCOVERY_EXTERNAL_ADDRESS")) {
  $env:SYNERGY_DISCOVERY_EXTERNAL_ADDRESS = $NodeEnv["DISCOVERY_EXTERNAL_ADDRESS"]
  $env:SYNERGY_DISCOVERY_PUBLIC_ADDRESS = $NodeEnv["DISCOVERY_EXTERNAL_ADDRESS"]
} elseif ($NodeEnv.ContainsKey("DISCOVERY_PUBLIC_ADDRESS")) {
  $env:SYNERGY_DISCOVERY_EXTERNAL_ADDRESS = $NodeEnv["DISCOVERY_PUBLIC_ADDRESS"]
  $env:SYNERGY_DISCOVERY_PUBLIC_ADDRESS = $NodeEnv["DISCOVERY_PUBLIC_ADDRESS"]
}

try {
  $proc = Start-Process -FilePath $BinPath -ArgumentList @("start", "--config", $ConfigPath) -WorkingDirectory $BaseDir -RedirectStandardOutput $OutFile -RedirectStandardError $ErrFile -PassThru -WindowStyle Hidden
}
finally {
  $env:SYNERGY_PROJECT_ROOT = $previousProjectRoot
  $env:SYNERGY_CONFIG_PATH = $previousConfigPath
  $env:SYNERGY_GENESIS_FILE = $previousGenesisPath
  $env:SYNERGY_BOOTSTRAP_ONLY = $previousBootstrapOnly
  $env:SYNERGY_AUTO_REGISTER_VALIDATOR = $previousAutoRegister
  $env:SYNERGY_P2P_PORT = $previousP2PPort
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
