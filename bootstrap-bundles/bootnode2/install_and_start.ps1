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
$DataDir = Join-Path $BaseDir "data"
$LogsDir = Join-Path $DataDir "logs"
$PidFile = Join-Path $DataDir "node.pid"
$OutFile = Join-Path $LogsDir "node.out"
$ErrFile = Join-Path $LogsDir "node.err"

New-Item -ItemType Directory -Force -Path $LogsDir | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $DataDir "chain") | Out-Null

if (-not (Test-Path $BinPath)) { throw "Missing Windows binary: $BinPath" }

$proc = Start-Process -FilePath $BinPath -ArgumentList @("start", "--config", $ConfigPath) -WorkingDirectory $BaseDir -RedirectStandardOutput $OutFile -RedirectStandardError $ErrFile -PassThru -WindowStyle Hidden
$proc.Id | Set-Content -Path $PidFile
"Started $($NodeEnv["MACHINE_ID"]) as bootstrap-only discovery node (PID $($proc.Id))"
