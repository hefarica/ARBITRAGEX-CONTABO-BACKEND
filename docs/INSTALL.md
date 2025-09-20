# InstalaciÃ³n y ConfiguraciÃ³n

## Requisitos Previos

- Docker y Docker Compose
- Git
- Acceso a Internet para descargar dependencias

## InstalaciÃ³n en Contabo VPS (ProducciÃ³n)

1. **Clonar el repositorio:**
   `ash
   git clone https://github.com/hefarica/ARBITRAGEX-CONTABO-BACKEND.git
   cd ARBITRAGEX-CONTABO-BACKEND
   `

2. **Configurar variables de entorno:**
   `ash
   cp .env.example .env
   # Editar .env con los valores adecuados
   `

3. **Iniciar servicios:**
   `ash
   cd infrastructure/docker
   docker-compose up -d
   `

## InstalaciÃ³n en Windows (Desarrollo)

1. **Requisitos:**
   - Docker Desktop para Windows con WSL 2
   - Git para Windows

2. **InstalaciÃ³n:**
   `powershell
   git clone https://github.com/hefarica/ARBITRAGEX-CONTABO-BACKEND.git
   cd ARBITRAGEX-CONTABO-BACKEND
   
   # Configurar entorno
   Copy-Item .env.example .env
   # Editar .env con los valores para desarrollo
   
   # Iniciar servicios
   cd infrastructure/docker
   docker-compose up -d
   `

## VerificaciÃ³n de la InstalaciÃ³n

Una vez iniciados los servicios, puedes verificar que todo funciona correctamente:

- API REST: http://localhost:3000/health
- WebSocket: ws://localhost:3001
- PostgreSQL: localhost:5432
- Redis: localhost:6379
- Grafana: http://localhost:3100

## SoluciÃ³n de Problemas

Ver [documentaciÃ³n de troubleshooting](./TROUBLESHOOTING.md) para problemas comunes.
