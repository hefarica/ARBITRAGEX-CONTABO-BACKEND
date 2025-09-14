param(
  [switch]$DryRun,     # solo mostrar qué se va a copiar
  [switch]$Force       # sobrescribir sin preguntar
)

# ===== Configuración =====
$ErrorActionPreference = "Continue"
$ProgressPreference = "Continue"

# ===== Utilidades =====
function Write-OK($t)     { Write-Host "✔ $t" -ForegroundColor Green }
function Write-Warn($t)   { Write-Warning $t }
function Write-Err($t)    { Write-Host "✖ $t" -ForegroundColor Red }
function Write-Info($t)   { Write-Host "→ $t" -ForegroundColor Yellow }

# Crear logs
New-Item -ItemType Directory -Force -Path "logs" | Out-Null
$logFile = ".\logs\copy-files-$(Get-Date -Format yyyyMMdd-HHmmss).log"
Start-Transcript -Path $logFile -Append | Out-Null

Write-Host "=== ArbitrageX Project Files Migration ===" -ForegroundColor Cyan
Write-Host "Copiando archivos desde directorio padre al repositorio Git" -ForegroundColor White

# ===== Definir archivos/carpetas a copiar =====
$sourceBase = ".."
$targetBase = "."

$copyItems = @(
    # Servicios Rust principales
    @{ Source = "$sourceBase/selector-api"; Target = "$targetBase/selector-api"; Type = "Directory" }
    @{ Source = "$sourceBase/sim-ctl"; Target = "$targetBase/sim-ctl"; Type = "Directory" }
    @{ Source = "$sourceBase/relays-client"; Target = "$targetBase/relays-client"; Type = "Directory" }
    @{ Source = "$sourceBase/recon"; Target = "$targetBase/recon"; Type = "Directory" }
    
    # Infraestructura
    @{ Source = "$sourceBase/docker/docker-compose.yml"; Target = "$targetBase/docker/docker-compose.yml"; Type = "File" }
    @{ Source = "$sourceBase/docker/docker-compose.prod.yml"; Target = "$targetBase/docker/docker-compose.prod.yml"; Type = "File" }
    @{ Source = "$sourceBase/docker/.dockerignore"; Target = "$targetBase/docker/.dockerignore"; Type = "File" }
    
    # Nginx
    @{ Source = "$sourceBase/nginx/nginx.conf"; Target = "$targetBase/nginx/nginx.conf"; Type = "File" }
    @{ Source = "$sourceBase/nginx/sites-available"; Target = "$targetBase/nginx/sites-available"; Type = "Directory" }
    
    # PostgreSQL
    @{ Source = "$sourceBase/postgresql/schema.sql"; Target = "$targetBase/postgresql/schema.sql"; Type = "File" }
    
    # Monitoring
    @{ Source = "$sourceBase/monitoring/prometheus.yml"; Target = "$targetBase/monitoring/prometheus.yml"; Type = "File" }
    @{ Source = "$sourceBase/monitoring/alerts.rules"; Target = "$targetBase/monitoring/alerts.rules"; Type = "File" }
    
    # Scripts (excepto los que ya están)
    @{ Source = "$sourceBase/scripts/deploy.sh"; Target = "$targetBase/scripts/deploy.sh"; Type = "File" }
    @{ Source = "$sourceBase/scripts/backup_db.sh"; Target = "$targetBase/scripts/backup_db.sh"; Type = "File" }
    @{ Source = "$sourceBase/scripts/health_check.sh"; Target = "$targetBase/scripts/health_check.sh"; Type = "File" }
    
    # Security
    @{ Source = "$sourceBase/security/firewall_rules.sh"; Target = "$targetBase/security/firewall_rules.sh"; Type = "File" }
    @{ Source = "$sourceBase/security/ssl_setup.sh"; Target = "$targetBase/security/ssl_setup.sh"; Type = "File" }
    
    # Contratos Smart
    @{ Source = "$sourceBase/contracts"; Target = "$targetBase/contracts"; Type = "Directory" }
    
    # Tests
    @{ Source = "$sourceBase/tests"; Target = "$targetBase/tests"; Type = "Directory" }
    @{ Source = "$sourceBase/test"; Target = "$targetBase/test"; Type = "Directory" }
    
    # Documentación principal
    @{ Source = "$sourceBase/DEPLOYMENT_GUIDE.md"; Target = "$targetBase/DEPLOYMENT_GUIDE.md"; Type = "File" }
)

# ===== Función para copiar item =====
function Copy-ProjectItem($item) {
    $source = $item.Source
    $target = $item.Target
    $type = $item.Type
    
    if (-not (Test-Path $source)) {
        Write-Warn "Fuente no existe: $source"
        return
    }
    
    # Crear directorio padre del target si no existe
    $targetDir = Split-Path $target -Parent
    if ($targetDir -and -not (Test-Path $targetDir)) {
        New-Item -ItemType Directory -Force -Path $targetDir | Out-Null
        Write-Info "Creado directorio: $targetDir"
    }
    
    if ($DryRun) {
        Write-Info "DRY RUN: $source → $target ($type)"
        return
    }
    
    try {
        if ($type -eq "Directory") {
            Copy-Item -Path $source -Destination $target -Recurse -Force:$Force
            Write-OK "Copiada carpeta: $source → $target"
        } else {
            Copy-Item -Path $source -Destination $target -Force:$Force
            Write-OK "Copiado archivo: $source → $target"
        }
    } catch {
        Write-Err "Error copiando $source → $target : $($_.Exception.Message)"
    }
}

# ===== Procesar todos los items =====
$totalItems = $copyItems.Count
$currentItem = 0

foreach ($item in $copyItems) {
    $currentItem++
    $percentComplete = [int](($currentItem / $totalItems) * 100)
    Write-Progress -Activity "Copiando archivos del proyecto" -Status "Item $currentItem de $totalItems" -PercentComplete $percentComplete
    
    Copy-ProjectItem $item
}

Write-Progress -Activity "Copiando archivos del proyecto" -Completed

# ===== Crear .env.example si no existe =====
if (-not (Test-Path ".env.example") -and -not $DryRun) {
    Write-Info "Creando .env.example..."
    @"
# ArbitrageX Supreme v3.0 - Environment Variables
# Copy this file to .env and update with your values

# Database
POSTGRES_USER=arbitragex
POSTGRES_PASSWORD=ChangeThisPassword123!
POSTGRES_DB=arbitragex

# Redis
REDIS_PASSWORD=ChangeThisRedisPassword123!

# JWT
JWT_SECRET=your-super-secret-jwt-key-change-in-production

# Ethereum Node
RPC_URL=http://geth-node:8545
WS_URL=ws://geth-node:8546

# MinIO
MINIO_ROOT_USER=arbitragex
MINIO_ROOT_PASSWORD=ChangeThisMinioPassword123!

# Activepieces
AP_ENCRYPTION_KEY=your-32-character-encryption-key!
AP_JWT_SECRET=your-activepieces-jwt-secret

# Grafana
GRAFANA_USER=admin
GRAFANA_PASSWORD=ChangeThisGrafanaPassword123!

# Environment
ENVIRONMENT=development

# Feature Flags
ENABLE_METRICS=true
ENABLE_TRACING=true
ENABLE_PROFILING=false
"@ | Out-File -FilePath ".env.example" -Encoding utf8
    Write-OK "Creado .env.example"
}

# ===== Reporte final =====
Write-Host "`n=== Migración completada ===" -ForegroundColor Cyan

if ($DryRun) {
    Write-Host "DRY RUN completado. Ejecuta sin -DryRun para aplicar cambios." -ForegroundColor Yellow
} else {
    Write-OK "Archivos copiados exitosamente"
    Write-Host "Log: $logFile" -ForegroundColor Gray
    
    Write-Host "`nPróximos pasos:" -ForegroundColor Cyan
    Write-Host "1. git add ." -ForegroundColor Yellow
    Write-Host "2. git commit -m 'feat: implement backend infrastructure'" -ForegroundColor Yellow  
    Write-Host "3. git push origin master" -ForegroundColor Yellow
}

Stop-Transcript | Out-Null
