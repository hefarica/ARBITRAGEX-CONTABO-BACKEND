//! # Módulo de configuración
//!
//! Gestiona la configuración del sistema, cargando valores desde
//! variables de entorno o archivos de configuración.

use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use serde::{Deserialize, Serialize};

/// Estructura principal de configuración
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Host donde se ejecutará el servidor WebSocket
    pub host: String,
    /// Puerto para el servidor WebSocket
    pub port: u16,
    /// Ruta del archivo de log
    pub log_file_path: String,
    /// Intervalo en segundos para el monitoreo
    pub monitoring_interval_sec: u64,
    /// Número máximo de peticiones concurrentes
    pub max_concurrent_requests: usize,
    /// Tamaño de cada grupo de procesamiento
    pub chunk_size: usize,
    /// Retraso en milisegundos entre grupos
    pub chunk_delay_ms: u64,
    /// Número mínimo de endpoints para considerar el sistema como operativo
    pub min_operational_endpoints: usize,
    /// Umbrales para clasificación de latencia
    pub latency_thresholds: LatencyThresholds,
}

/// Umbrales para clasificar la latencia
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyThresholds {
    /// Latencia máxima para considerar un endpoint como "online" (ms)
    pub online_ms: u64,
    /// Latencia máxima para considerar un endpoint como "degraded" (ms)
    pub degraded_ms: u64,
    /// Latencia máxima para considerar un endpoint como "slow" (ms)
    pub slow_ms: u64,
}

impl Default for LatencyThresholds {
    fn default() -> Self {
        LatencyThresholds {
            online_ms: 100,
            degraded_ms: 300,
            slow_ms: 1000,
        }
    }
}

impl Config {
    /// Carga la configuración desde variables de entorno o archivo de configuración
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        // Intentar cargar desde archivo primero
        if let Ok(config) = Self::load_from_file() {
            return Ok(config);
        }

        // Si no hay archivo, cargar desde variables de entorno
        let host = env::var("ARBITRAGEX_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = env::var("ARBITRAGEX_PORT")
            .unwrap_or_else(|_| "3030".to_string())
            .parse::<u16>()
            .unwrap_or(3030);
        
        let log_file_path = env::var("ARBITRAGEX_LOG_PATH")
            .unwrap_or_else(|_| "audit.log".to_string());
        
        let monitoring_interval_sec = env::var("ARBITRAGEX_MONITOR_INTERVAL")
            .unwrap_or_else(|_| "1".to_string())
            .parse::<u64>()
            .unwrap_or(1);
        
        let max_concurrent_requests = env::var("ARBITRAGEX_MAX_CONCURRENT")
            .unwrap_or_else(|_| "20".to_string())
            .parse::<usize>()
            .unwrap_or(20);
        
        let chunk_size = env::var("ARBITRAGEX_CHUNK_SIZE")
            .unwrap_or_else(|_| "3".to_string())
            .parse::<usize>()
            .unwrap_or(3);
        
        let chunk_delay_ms = env::var("ARBITRAGEX_CHUNK_DELAY")
            .unwrap_or_else(|_| "350".to_string())
            .parse::<u64>()
            .unwrap_or(350);
        
        let min_operational_endpoints = env::var("ARBITRAGEX_MIN_ENDPOINTS")
            .unwrap_or_else(|_| "5".to_string())
            .parse::<usize>()
            .unwrap_or(5);
        
        // Latency thresholds
        let online_ms = env::var("ARBITRAGEX_LATENCY_ONLINE")
            .unwrap_or_else(|_| "100".to_string())
            .parse::<u64>()
            .unwrap_or(100);
        
        let degraded_ms = env::var("ARBITRAGEX_LATENCY_DEGRADED")
            .unwrap_or_else(|_| "300".to_string())
            .parse::<u64>()
            .unwrap_or(300);
        
        let slow_ms = env::var("ARBITRAGEX_LATENCY_SLOW")
            .unwrap_or_else(|_| "1000".to_string())
            .parse::<u64>()
            .unwrap_or(1000);

        Ok(Config {
            host,
            port,
            log_file_path,
            monitoring_interval_sec,
            max_concurrent_requests,
            chunk_size,
            chunk_delay_ms,
            min_operational_endpoints,
            latency_thresholds: LatencyThresholds {
                online_ms,
                degraded_ms,
                slow_ms,
            },
        })
    }

    /// Carga la configuración desde un archivo JSON
    fn load_from_file() -> Result<Self, Box<dyn std::error::Error>> {
        // Buscar archivo de configuración en varias ubicaciones
        let config_paths = [
            "config.json",
            "config/config.json",
            "/etc/arbitragex/config.json",
        ];

        for path in &config_paths {
            if Path::new(path).exists() {
                let mut file = File::open(path)?;
                let mut contents = String::new();
                file.read_to_string(&mut contents)?;
                let config: Config = serde_json::from_str(&contents)?;
                return Ok(config);
            }
        }

        Err("No config file found".into())
    }

    /// Genera una configuración por defecto
    pub fn default_for_environment(env_type: &str) -> Self {
        match env_type {
            "production" => Config {
                host: "0.0.0.0".to_string(), // Escucha en todas las interfaces
                port: 3030,
                log_file_path: "/home/arbitragex/ARBITRAGEX-CONTABO-BACKEND/searcher-rs/audit.log".to_string(),
                monitoring_interval_sec: 1,
                max_concurrent_requests: 30,
                chunk_size: 5,
                chunk_delay_ms: 200,
                min_operational_endpoints: 10,
                latency_thresholds: LatencyThresholds {
                    online_ms: 100,
                    degraded_ms: 300,
                    slow_ms: 1000,
                },
            },
            "development" => Config {
                host: "127.0.0.1".to_string(),
                port: 3030,
                log_file_path: "audit.log".to_string(),
                monitoring_interval_sec: 1,
                max_concurrent_requests: 10,
                chunk_size: 3,
                chunk_delay_ms: 350,
                min_operational_endpoints: 3,
                latency_thresholds: LatencyThresholds {
                    online_ms: 100,
                    degraded_ms: 300,
                    slow_ms: 1000,
                },
            },
            _ => Config {
                host: "127.0.0.1".to_string(),
                port: 3030,
                log_file_path: "audit.log".to_string(),
                monitoring_interval_sec: 1,
                max_concurrent_requests: 10,
                chunk_size: 3,
                chunk_delay_ms: 350,
                min_operational_endpoints: 3,
                latency_thresholds: LatencyThresholds::default(),
            },
        }
    }

    /// Guarda la configuración actual a un archivo
    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}
