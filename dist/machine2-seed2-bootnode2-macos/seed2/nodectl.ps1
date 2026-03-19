$ErrorActionPreference = "Stop"

$BaseDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$PidFile = Join-Path $BaseDir "data/seed.pid"
$OutFile = Join-Path $BaseDir "data/logs/seed.out"

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
      "Stopped seed service"
    } else {
      "Seed service is not running"
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
