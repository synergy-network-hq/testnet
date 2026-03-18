$ErrorActionPreference = "Stop"
$BaseDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$Py = Get-Command python3 -ErrorAction SilentlyContinue
if (-not $Py) { $Py = Get-Command python -ErrorAction SilentlyContinue }
if (-not $Py) { throw "Python is required to run the seed service." }

$DataDir = Join-Path $BaseDir "data"
$LogsDir = Join-Path $DataDir "logs"
$PidFile = Join-Path $DataDir "seed.pid"
$OutFile = Join-Path $LogsDir "seed.out"
$ErrFile = Join-Path $LogsDir "seed.err"

New-Item -ItemType Directory -Force -Path $LogsDir | Out-Null
$proc = Start-Process -FilePath $Py.Source -ArgumentList @((Join-Path $BaseDir "seed_service.py")) -WorkingDirectory $BaseDir -RedirectStandardOutput $OutFile -RedirectStandardError $ErrFile -PassThru -WindowStyle Hidden
$proc.Id | Set-Content -Path $PidFile
"Started seed service PID $($proc.Id)"
