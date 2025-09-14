param(
  [switch]$Fix,
  [switch]$NonInteractive
)

$ErrorActionPreference = "Stop"
$ProgressPreference    = "Continue"
New-Item -ItemType Directory -Force -Path "logs" | Out-Null
$logFile = Join-Path $PWD "logs\bootstrap-$(Get-Date -Format yyyyMMdd-HHmmss).log"
Start-Transcript -Path $logFile -Append | Out-Null

function Step([string]$Name, [scriptblock]$Body) {
  Write-Host ""
  Write-Host ">> $Name"
  try {
    & $Body 2>&1 | Tee-Object -FilePath ".\logs\bootstrap-stream.log" -Append
    Write-Host "OK: $Name"
  } catch {
    Write-Host "FAIL: $Name - $($_.Exception.Message)"
    Stop-Transcript | Out-Null
    exit 1
  }
}

function Require-Tool($label, $cmd) {
  if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) {
    if ($Fix) {
      Write-Warning "$label no encontrado. Instala antes de continuar."
    } else {
      throw "$label no encontrado. Ejecuta con -Fix para continuar con advertencia o instala la herramienta."
    }
  } else {
    Write-Host "Found $label"
  }
}

Step "ExecutionPolicy (bypass en el proceso)" {
  Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force
}
Step "Desbloquear archivos descargados" {
  Get-ChildItem -Recurse -Include *.ps1,*.psm1,*.cmd,*.bat,*.sh -ErrorAction SilentlyContinue |
    Unblock-File -ErrorAction SilentlyContinue
}
Step "Crear carpetas base si faltan" {
  @("docker","nginx","scripts","monitoring","postgresql","security","relays-client","recon") |
    ForEach-Object { if (-not (Test-Path $_)) { New-Item -Type Directory $_ | Out-Null } }
}

Step "Comprobar herramientas basicas" {
  Require-Tool "Git" "git"
  Require-Tool "Node" "node"
  Require-Tool "Docker" "docker"
  Require-Tool "docker compose (v2)" "docker"
}

Step "Docker en ejecucion" {
  docker info | Out-Null
}

Step "Grupo docker-users (solo Windows con Docker Desktop)" {
  $list = (net localgroup docker-users 2>$null)
  if ($LASTEXITCODE -ne 0) { Write-Host "Grupo docker-users no encontrado (Docker no instalado o antiguo). Continuo."; return }
  $user = $env:USERNAME
  $in = (net localgroup docker-users 2>$null | Select-String -SimpleMatch $user)
  if (-not $in) {
    if ($Fix) {
      $isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()
                 ).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
      if (-not $isAdmin) {
        Write-Warning "Ejecuta este script como Administrador para agregarte al grupo docker-users."
      } else {
        cmd /c "net localgroup docker-users $user /add" | Out-Null
        Write-Host "Agregado $user a docker-users. Cierra sesion y vuelve a entrar para aplicar."
      }
    } else {
      Write-Warning "$user no pertenece a docker-users."
    }
  } else {
    Write-Host "docker-users OK"
  }
}

Step "Conectividad TLS basica" {
  foreach($h in "github.com","registry-1.docker.io","api.cloudflare.com"){
    try {
      $r = Test-NetConnection $h -Port 443 -WarningAction SilentlyContinue
      if(-not $r.TcpTestSucceeded){ throw "Sin acceso a $h:443" }
      Write-Host "OK TLS $h"
    } catch { Write-Warning $_.Exception.Message }
  }
}

Step "Modo no interactivo y salida verbosa" {
  $env:CI="1"
  $env:NODE_DISABLE_COLORS="0"
  Write-Host "CI=1 activado"
}

Write-Host ""
Write-Host "Bootstrap completado. Log: $logFile"
Stop-Transcript | Out-Null
exit 0