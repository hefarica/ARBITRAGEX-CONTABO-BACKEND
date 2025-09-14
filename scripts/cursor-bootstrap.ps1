param(
  [switch]$Fix,               # intenta reparar (requiere admin para algunas acciones)
  [switch]$NonInteractive     # no pedir confirmaciones
)

# ===== Salida visible y robusta =====
$ErrorActionPreference = "Stop"
$ProgressPreference    = "Continue"
$InformationPreference = "Continue"
$VerbosePreference     = "Continue"
$PSStyle.OutputRendering = "Ansi"  # colores

# ===== Logging =====
New-Item -ItemType Directory -Force -Path "logs" | Out-Null
$logFile = ".\logs\bootstrap-$(Get-Date -Format yyyyMMdd-HHmmss).log"
Start-Transcript -Path $logFile -Append | Out-Null

# ===== Utilidades =====
function Write-Section($t) { Write-Host "`n== $t ==" -ForegroundColor Cyan }
function Write-OK($t)     { Write-Host "✔ $t" -ForegroundColor Green }
function Write-Warn($t)   { Write-Warning $t }
function Write-Err($t)    { Write-Host "✖ $t" -ForegroundColor Red }

function Test-Admin {
  ([Security.Principal.WindowsPrincipal]
   [Security.Principal.WindowsIdentity]::GetCurrent()
  ).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Invoke-Step([string]$Name,[scriptblock]$Block) {
  Write-Progress -Activity "Bootstrap" -Status $Name -PercentComplete 0
  Write-Host "`n→ $Name" -ForegroundColor Yellow
  try {
    & $Block 2>&1 | Tee-Object -FilePath ".\logs\bootstrap-stream.log" -Append
    Write-OK $Name
  } catch {
    Write-Err "$Name :: $($_.Exception.Message)"
    Stop-Transcript | Out-Null
    exit 2
  } finally {
    Write-Progress -Activity "Bootstrap" -Completed
  }
}

function Require-Tool($cmd, $how) {
  if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) {
    if ($Fix) { Write-Warn "Falta '$cmd'. $how"; }
    else {
      Write-Err "Falta '$cmd'. Ejecuta con -Fix o instálalo: $how"
      Stop-Transcript | Out-Null
      exit 3
    }
  } else { Write-OK "$cmd OK" }
}

# ===== Pre-políticas y desbloqueos =====
Invoke-Step "ExecutionPolicy (Bypass en el proceso)" {
  Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force
}
Invoke-Step "Desbloquear archivos descargados (MoTW)" {
  Get-ChildItem -Recurse -Include *.ps1,*.psm1,*.cmd,*.bat,*.sh -ErrorAction SilentlyContinue |
    Unblock-File -ErrorAction SilentlyContinue
}

if ($Fix) {
  Invoke-Step "ExecutionPolicy (RemoteSigned para el usuario)" {
    Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned -Force
  }
}

# ===== Requisitos base =====
Write-Section "Requisitos"
Require-Tool git "Instala Git: https://git-scm.com/download/win"
Require-Tool pwsh "Instala PowerShell 7+: https://aka.ms/powershell-release"
Require-Tool node "Instala Node LTS: https://nodejs.org/ (o winget install OpenJS.NodeJS.LTS)"
Require-Tool docker "Instala Docker Desktop y arráncalo"
Require-Tool docker-compose "Docker Desktop incluye compose v2"
Require-Tool cargo "Instala Rust: https://rustup.rs/"
Require-Tool wrangler "npm i -g wrangler"
Require-Tool psql "Instala PostgreSQL client"
Require-Tool redis-cli "Instala Redis/Valkey CLI (o winget install valkey)"

# ===== Servicios locales / permisos =====
Invoke-Step "Docker en ejecución" {
  docker info | Out-Null
}

Invoke-Step "Membership docker-users" {
  $user = "$env:UserName"
  $inGroup = (net localgroup docker-users 2>$null | Select-String -SimpleMatch $user) -ne $null
  if (-not $inGroup) {
    if ($Fix) {
      if (-not (Test-Admin)) { throw "Se requiere PowerShell como Administrador para añadir a docker-users." }
      cmd /c "net localgroup docker-users ""$user"" /add" | Out-Null
      Write-Warn "Se añadió $user a 'docker-users'. Cierra sesión y vuelve a entrar para aplicar."
    } else {
      Write-Warn "$user no está en docker-users. Ejecuta con -Fix (admin) o añade manualmente."
    }
  } else { Write-OK "docker-users OK" }
}

# ===== Conectividad =====
Invoke-Step "Red: GitHub/Cloudflare/Registries" {
  foreach ($host in @("github.com","registry-1.docker.io","api.cloudflare.com")) {
    try {
      $ok = Test-NetConnection $host -Port 443 -WarningAction SilentlyContinue
      if (-not $ok.TcpTestSucceeded) { throw "No hay TLS con $host:443" }
      Write-OK "Conectividad TLS $host OK"
    } catch {
      Write-Warn "Fallo comprobando $host: $($_.Exception.Message) (continuo)"
    }
  }
}

# ===== Node/npm configuración de logs verbosos para el agente =====
Invoke-Step "Modo verboso para herramientas comunes" {
  $env:NODE_DISABLE_COLORS = "0"
  $env:CI = "1"             # evita prompts interactivos
  Write-OK "CI=1; salida no interactiva activada"
}

# ===== Validación de versiones mínimas =====
Invoke-Step "Versiones mínimas" {
  $min = @{
    node = "18.0.0"
    git  = "2.30.0"
  }
  function Parse($v){ [version]($v -replace '[^\d\.]','') }
  $nv = Parse ((node -v) 2>$null)
  $gv = Parse ((git --version) 2>$null)
  if ($nv -lt (Parse $min.node)) { throw "Node >= $($min.node) requerido. Actual: $nv" }
  if ($gv -lt (Parse $min.git))  { throw "Git  >= $($min.git)  requerido. Actual: $gv" }
}

# ===== Postgres / Valkey =====
Invoke-Step "Postgres reachable (si corre en docker compose: postgres:5432)" {
  try {
    $pgUrl = $env:POSTGRES_URL
    if ($pgUrl) { Write-Host "POSTGRES_URL=$pgUrl" -ForegroundColor DarkGray }
  } catch {}
}
Invoke-Step "Valkey/Redis reachable" {
  try {
    $r = & redis-cli PING 2>$null
    if ($r -ne "PONG") { Write-Warn "redis-cli no responde PONG (continuo)" }
  } catch { Write-Warn "redis-cli no disponible (continuo)" }
}

# ===== Reporte final =====
Write-Section "Listo"
Write-OK "Entorno validado. Logs: $logFile"
Write-Host "Ejecuta tus comandos con salida en vivo desde Terminal." -ForegroundColor Cyan

Stop-Transcript | Out-Null
exit 0
