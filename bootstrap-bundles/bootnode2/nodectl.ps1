$ErrorActionPreference = "Stop"

$BaseDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$EnvPath = Join-Path $BaseDir "node.env"
$NodeEnv = @{}
Get-Content $EnvPath | ForEach-Object {
  if ($_ -match '^\s*$' -or $_ -match '^\s*#') { return }
  $parts = $_ -split '=', 2
  if ($parts.Count -eq 2) { $NodeEnv[$parts[0].Trim()] = $parts[1].Trim() }
}

$PidFile = Join-Path $BaseDir "data/node.pid"
$OutFile = Join-Path $BaseDir "data/logs/node.out"

function Test-Running {
  if (-not (Test-Path $PidFile)) { return $false }
  $pid = Get-Content $PidFile
  return [bool](Get-Process -Id $pid -ErrorAction SilentlyContinue)
}

switch ($args[0]) {
  "start" { & (Join-Path $BaseDir "install_and_start.ps1") }
  "stop" {
    if (Test-Running) {
      $pid = Get-Content $PidFile
      Stop-Process -Id $pid -Force
      Remove-Item $PidFile -Force -ErrorAction SilentlyContinue
      "Stopped $($NodeEnv["MACHINE_ID"])"
    } else {
      "Node is not running"
    }
  }
  "status" {
    if (Test-Running) {
      "Running (PID $(Get-Content $PidFile))"
    } else {
      "Stopped"
    }
  }
  "logs" {
    if ($args.Count -gt 1 -and $args[1] -eq "--follow") {
      Get-Content $OutFile -Wait
    } else {
      Get-Content $OutFile -Tail 120
    }
  }
  default {
    "Usage: .\nodectl.ps1 <start|stop|status|logs>"
    exit 1
  }
}
