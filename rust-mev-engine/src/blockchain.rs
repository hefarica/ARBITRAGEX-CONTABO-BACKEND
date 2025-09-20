//! # Módulo de blockchain
//!
//! Contiene las estructuras y funciones relacionadas con la interacción con blockchains.
//! Implementa el patrón Circuit Breaker para manejar fallas en endpoints.

use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

/// Estructura para datos de una cadena de bloques
#[derive(Debug, Clone)]
pub struct ChainData {
    pub name: String,
    pub slug: String,
    pub url: String,
    pub block_number: Option<u64>,
    pub latency_ms: u64,
    pub status: String,
    pub success_count: u64,
    pub total_count: u64,
    pub last_updated: u64,
    pub circuit_breaker: CircuitBreaker,
}

impl ChainData {
    /// Crea una nueva instancia de ChainData
    pub fn new(name: String, slug: String, url: String) -> Self {
        ChainData {
            name,
            slug,
            url,
            block_number: None,
            latency_ms: 9999, // Valor inicial hasta primera medición real
            status: "unknown".to_string(),
            success_count: 0,
            total_count: 0,
            last_updated: 0,
            circuit_breaker: CircuitBreaker::new(),
        }
    }

    /// Calcula el uptime como porcentaje basado en éxitos y totales
    pub fn calculate_uptime(&self) -> f32 {
        if self.total_count > 0 {
            (self.success_count as f32 / self.total_count as f32) * 100.0
        } else {
            0.0
        }
    }
}

/// Implementación del patrón Circuit Breaker para manejar endpoints inestables
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    pub state: String,
    pub failures: u32,
    pub last_failure: Option<Instant>,
    pub threshold: u32,
    pub timeout: Duration,
}

impl CircuitBreaker {
    /// Crea un nuevo Circuit Breaker
    pub fn new() -> Self {
        CircuitBreaker {
            state: "closed".to_string(),
            failures: 0,
            last_failure: None,
            threshold: 3, // 3 fallos seguidos para abrir el circuito
            timeout: Duration::from_secs(30), // 30 segundos antes de reintentar
        }
    }

    /// Verifica si se puede realizar una llamada según el estado del circuito
    pub fn check(&mut self) -> bool {
        match self.state.as_str() {
            "closed" => true, // Circuito cerrado, permitir llamadas
            "open" => {
                if let Some(last_failure) = self.last_failure {
                    if last_failure.elapsed() > self.timeout {
                        // Han pasado más de 30 segundos, intentar de nuevo en modo half-open
                        self.state = "half-open".to_string();
                        true
                    } else {
                        // Aún no ha pasado suficiente tiempo, mantener circuito abierto
                        false
                    }
                } else {
                    // No debería llegar aquí, pero si ocurre, permitir la llamada
                    true
                }
            }
            "half-open" => true, // Permitir una llamada de prueba
            _ => true, // Estado desconocido, permitir por defecto
        }
    }

    /// Registra un éxito, cerrando el circuito si estaba en half-open
    pub fn record_success(&mut self) {
        if self.state == "half-open" {
            self.state = "closed".to_string();
        }
        self.failures = 0;
    }

    /// Registra un fallo, abriendo el circuito si se alcanza el umbral
    pub fn record_failure(&mut self) {
        self.failures += 1;
        self.last_failure = Some(Instant::now());
        if self.failures >= self.threshold {
            self.state = "open".to_string();
        }
    }
}

/// Estructura para respuestas de blockchain
#[derive(Debug, Serialize, Deserialize)]
pub struct ChainResponse {
    pub chain_slug: String,
    pub data: ChainResponseData,
}

/// Datos detallados de respuesta de blockchain
#[derive(Debug, Serialize, Deserialize)]
pub struct ChainResponseData {
    pub name: String,
    pub block_number: u64,
    pub latency_ms: u64,
    pub uptime: f32,
    pub score: f32,
    pub last_updated: u64,
    pub status: String,
}
