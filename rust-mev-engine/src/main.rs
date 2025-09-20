//! # ArbitrageX Supreme V3.2 - Motor de monitoreo RPC
//! 
//! Motor principal del sistema de monitoreo de blockchain en tiempo real.
//! Este componente se encarga de:
//! - Monitorear múltiples blockchains de forma concurrente
//! - Calcular latencia, uptime y confiabilidad de cada RPC
//! - Implementar Circuit Breaker para endpoints inestables
//! - Transmitir datos en tiempo real vía WebSocket
//! - Generar logs de auditoría para todas las operaciones

use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::sleep;
use warp::ws::{Message, WebSocket};
use warp::Filter;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use chrono::{DateTime, Utc};

// Módulos internos
mod blockchain;
mod config;
mod websocket;
mod utils;
mod monitoring;

use config::Config;
use blockchain::{ChainData, ChainResponse, ChainResponseData, CircuitBreaker};
use websocket::WebSocketManager;
use utils::logging;

/// Tipo para gestionar clientes WebSocket conectados
type Clients = Arc<Mutex<HashMap<String, mpsc::UnboundedSender<Result<Message, warp::Error>>>>>;

/// Obtiene la lista de endpoints de blockchain a monitorear
fn get_chain_endpoints() -> Vec<(String, String, String)> {
    vec![
        // Tier 1 - Alta prioridad (2s)
        ("Ethereum".to_string(), "ethereum".to_string(), "https://cloudflare-eth.com".to_string()),
        ("Arbitrum One".to_string(), "arbitrum".to_string(), "https://arbitrum-one-rpc.publicnode.com".to_string()),
        ("Polygon".to_string(), "polygon".to_string(), "https://polygon-bor-rpc.publicnode.com".to_string()),
        ("Base".to_string(), "base".to_string(), "https://base.llamarpc.com".to_string()),
        ("Optimism".to_string(), "optimism".to_string(), "https://optimism.publicnode.com".to_string()),
        ("BNB Chain".to_string(), "bnb".to_string(), "https://bsc.publicnode.com".to_string()),
        ("Avalanche".to_string(), "avalanche".to_string(), "https://avalanche.publicnode.com".to_string()),
        ("Fantom".to_string(), "fantom".to_string(), "https://fantom.publicnode.com".to_string()),
        
        // Tier 2 - Media prioridad (4s)
        ("zkSync Era".to_string(), "zksync".to_string(), "https://zksync.publicnode.com".to_string()),
        ("Linea".to_string(), "linea".to_string(), "https://linea.publicnode.com".to_string()),
        ("Scroll".to_string(), "scroll".to_string(), "https://scroll.publicnode.com".to_string()),
        ("Mode".to_string(), "mode".to_string(), "https://mode.publicnode.com".to_string()),
        ("Taiko".to_string(), "taiko".to_string(), "https://taiko.publicnode.com".to_string()),
        ("Blast".to_string(), "blast".to_string(), "https://blast.publicnode.com".to_string()),
        
        // Tier 3 - Baja prioridad (6s)
        ("Mantle".to_string(), "mantle".to_string(), "https://mantle.publicnode.com".to_string()),
        ("Arbitrum Nova".to_string(), "arbitrum-nova".to_string(), "https://arbitrum-nova-rpc.publicnode.com".to_string()),
        ("Polygon zkEVM".to_string(), "polygon-zkevm".to_string(), "https://polygon-zkevm.publicnode.com".to_string()),
        ("Canto".to_string(), "canto".to_string(), "https://canto.publicnode.com".to_string()),
        
        // Tier 4 - Nuevas/Experimentales (10s)
        ("Kava".to_string(), "kava".to_string(), "https://kava.publicnode.com".to_string()),
        ("Sei".to_string(), "sei".to_string(), "https://sei.publicnode.com".to_string()),
        
        // Más endpoints
        ("Gnosis Chain".to_string(), "gnosis".to_string(), "https://gnosis.publicnode.com".to_string()),
        ("Harmony".to_string(), "harmony".to_string(), "https://harmony.publicnode.com".to_string()),
        ("Moonbeam".to_string(), "moonbeam".to_string(), "https://moonbeam.publicnode.com".to_string()),
        ("Moonriver".to_string(), "moonriver".to_string(), "https://moonriver.publicnode.com".to_string()),
        ("Aurora".to_string(), "aurora".to_string(), "https://aurora.publicnode.com".to_string()),
        ("Cronos".to_string(), "cronos".to_string(), "https://cronos.publicnode.com".to_string()),
        ("Evmos".to_string(), "evmos".to_string(), "https://evmos.publicnode.com".to_string()),
        ("Astar".to_string(), "astar".to_string(), "https://astar.publicnode.com".to_string()),
    ]
}

/// Realiza una llamada RPC para obtener el número de bloque actual y la latencia
async fn fetch_block_number(url: &str) -> Result<(u64, u64), String> {
    let client = reqwest::Client::new();
    let start = std::time::Instant::now();
    
    // Llamada HTTP POST con JSON-RPC estándar
    match client.post(url)
        .header("Content-Type", "application/json")
        .body(r#"{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}"#)
        .timeout(Duration::from_secs(3))
        .send()
        .await {
        Ok(res) => {
            // Verificar estado HTTP
            if !res.status().is_success() { 
                return Err(format!("HTTP error: {}", res.status())); 
            }
            
            // Parsear respuesta JSON
            let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
            
            // Extraer y validar el número de bloque
            let block_hex = json["result"].as_str().ok_or("No result")?;
            let block = u64::from_str_radix(&block_hex[2..], 16).map_err(|_| "Invalid block")?;
            
            // Calcular latencia real
            let latency = start.elapsed().as_millis() as u64;
            
            Ok((block, latency))
        }
        Err(e) => Err(format!("Connection failed: {}", e)),
    }
}

/// Función para escribir en el archivo audit.log
fn log_to_file(message: &str, config: &Config) -> std::io::Result<()> {
    let log_path = &config.log_file_path;
    
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    writeln!(file, "{}", message)?;
    Ok(())
}

/// Calcular el score de confiabilidad de una cadena
fn calculate_chain_score(chain: &ChainData) -> f32 {
    // Puntuación de latencia - Mejor cuando es menor
    let latency_score = if chain.latency_ms < 1000 {
        (100.0 - (chain.latency_ms as f32 / 10.0)).max(0.0) / 100.0
    } else {
        0.0
    };

    // Calcular uptime real basado en éxitos/total
    let uptime = if chain.total_count > 0 {
        (chain.success_count as f32 / chain.total_count as f32) * 100.0
    } else {
        0.0
    };

    // Ponderación: 60% latencia, 40% uptime
    (latency_score * 0.6) + ((uptime / 100.0) * 0.4)
}

/// Manejador de conexiones WebSocket
async fn handle_ws_connection(ws: WebSocket, clients: Clients, id: String, config: Arc<Config>) {
    let (mut ws_tx, mut ws_rx) = ws.split();

    // Canal para enviar mensajes al cliente
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Almacenar el canal en el mapa de clientes
    clients.lock().unwrap().insert(id.clone(), tx);

    // Procesar mensajes recibidos del cliente
    tokio::task::spawn(async move {
        while let Some(result) = ws_rx.next().await {
            match result {
                Ok(_) => (), // Ignorar mensajes del cliente por ahora
                Err(e) => {
                    eprintln!("Error en websocket: {}", e);
                    break;
                }
            }
        }
    });

    // Log de conexión
    let _ = log_to_file(
        &format!("[{}] NEW CLIENT CONNECTED: {}", 
            chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ"), id),
        &config
    );

    // Enviar mensajes al cliente
    while let Some(message) = rx.recv().await {
        if let Err(e) = ws_tx.send(message?).await {
            eprintln!("Error enviando mensaje por websocket: {}", e);
            break;
        }
    }

    // Eliminar cliente cuando se desconecta
    clients.lock().unwrap().remove(&id);
    
    // Log de desconexión
    let _ = log_to_file(
        &format!("[{}] CLIENT DISCONNECTED: {}", 
            chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ"), id),
        &config
    );
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Cargar configuración
    let config = Arc::new(Config::load().expect("Failed to load configuration"));
    
    // Inicializar archivo audit.log
    let now: DateTime<Utc> = Utc::now();
    let _ = log_to_file(
        &format!("[{}] STARTING ARBITRAGEX SUPREME V3.2", now.format("%Y-%m-%dT%H:%M:%SZ")),
        &config
    );

    // Inicializar datos de las cadenas
    let chains = Arc::new(Mutex::new(get_chain_endpoints()
        .into_iter()
        .map(|(name, slug, url)| {
            (slug.clone(), ChainData::new(name, slug, url))
        })
        .collect::<HashMap<String, ChainData>>()));

    // Inicializar gestor de clientes WebSocket
    let clients = Arc::new(Mutex::new(HashMap::new()));
    let clients_clone = clients.clone();

    // Contador para IDs de clientes
    let next_id = Arc::new(AtomicUsize::new(1));

    // Ruta WebSocket
    let config_clone = config.clone();
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            let clients = clients_clone.clone();
            let id = next_id.fetch_add(1, Ordering::Relaxed).to_string();
            let config = config_clone.clone();
            ws.on_upgrade(move |socket| handle_ws_connection(socket, clients, id, config))
        });

    // Iniciar el servidor
    println!("🚀 ArbitrageX Supreme V3.2 Backend iniciado en ws://{}:{}/ws", 
        config.host, config.port);
    
    let _ = log_to_file(
        &format!("[{}] SERVER STARTED: ws://{}:{}/ws", 
            now.format("%Y-%m-%dT%H:%M:%SZ"), config.host, config.port),
        &config
    );

    // Tarea para monitorear las cadenas
    let chains_clone = chains.clone();
    let clients_clone = clients.clone();
    let config_clone = config.clone();
    
    tokio::spawn(async move {
        // Sistema de prioridades dinámicas
        let priorities: HashMap<&str, u64> = [
            ("ethereum", 2), ("arbitrum", 2), ("polygon", 2), ("base", 2),
            ("optimism", 2), ("bnb", 2), ("avalanche", 2), ("fantom", 2),
            ("zksync", 4), ("linea", 4), ("scroll", 4), ("mode", 4),
            ("taiko", 4), ("blast", 4),
            ("mantle", 6), ("arbitrum-nova", 6), ("polygon-zkevm", 6), ("canto", 6),
            ("kava", 10), ("sei", 10),
        ].iter().cloned().collect();

        // Semáforo para limitar concurrencia
        let semaphore = Arc::new(tokio::sync::Semaphore::new(config_clone.max_concurrent_requests));

        loop {
            // Obtener todas las cadenas
            let chain_slugs: Vec<String> = chains_clone.lock().unwrap()
                .keys().cloned().collect();

            // Procesar en paralelo con límite de concurrencia
            let mut tasks = vec![];
            for chunk in chain_slugs.chunks(config_clone.chunk_size) {
                for slug in chunk {
                    let slug = slug.clone();
                    let chains = chains_clone.clone();
                    let priority = *priorities.get(slug.as_str()).unwrap_or(&10);
                    let config = config_clone.clone();
                    let semaphore = semaphore.clone();
                    
                    let task = tokio::spawn(async move {
                        // Adquirir un permiso del semáforo
                        let _permit = semaphore.acquire().await.unwrap();
                        
                        let chain_result = {
                            let mut chains_guard = chains.lock().unwrap();
                            let chain = chains_guard.get_mut(&slug).unwrap();
                            
                            // Verificar el circuit breaker
                            if !chain.circuit_breaker.check() {
                                return None;
                            }

                            // Incrementar el contador total
                            chain.total_count += 1;

                            // Realizar la llamada HTTP
                            match fetch_block_number(&chain.url).await {
                                Ok((block, latency)) => {
                                    // Actualizar datos
                                    chain.block_number = Some(block);
                                    chain.latency_ms = latency;
                                    chain.status = if latency < 100 { "online".to_string() } 
                                                else if latency < 300 { "degraded".to_string() } 
                                                else { "slow".to_string() };
                                    chain.success_count += 1;
                                    chain.last_updated = chrono::Utc::now().timestamp() as u64;
                                    
                                    // Registrar éxito en el circuit breaker
                                    chain.circuit_breaker.record_success();

                                    // Calcular uptime real
                                    let uptime = (chain.success_count as f32 / chain.total_count as f32) * 100.0;
                                    
                                    // Calcular score
                                    let score = calculate_chain_score(&chain);

                                    // Registrar en log
                                    let log_message = format!(
                                        "[{}] SUCCESS {} → {} → {}ms → uptime={:.2}%",
                                        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
                                        slug,
                                        block,
                                        latency,
                                        uptime
                                    );
                                    let _ = log_to_file(&log_message, &config);

                                    // Crear respuesta
                                    Some(ChainResponse {
                                        chain_slug: slug.clone(),
                                        data: ChainResponseData {
                                            name: chain.name.clone(),
                                            block_number: block,
                                            latency_ms: latency,
                                            uptime,
                                            score,
                                            last_updated: chain.last_updated,
                                            status: chain.status.clone(),
                                        },
                                    })
                                },
                                Err(e) => {
                                    // Registrar fallo
                                    chain.circuit_breaker.record_failure();
                                    chain.status = "offline".to_string();
                                    
                                    // Registrar en log
                                    let log_message = format!(
                                        "[{}] FAILED {} → {}",
                                        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
                                        slug,
                                        e
                                    );
                                    let _ = log_to_file(&log_message, &config);
                                    None
                                }
                            }
                        };

                        chain_result
                    });
                    tasks.push(task);
                }
                
                // Dormir para espaciar las solicitudes
                sleep(Duration::from_millis(config_clone.chunk_delay_ms)).await;
            }

            // Esperar resultados
            let mut results = vec![];
            for task in tasks {
                if let Ok(Some(result)) = task.await {
                    results.push(result);
                }
            }

            // Enviar resultados a clientes
            if !results.is_empty() {
                let json_results = json!(results);
                let message = Message::text(json_results.to_string());
                
                let clients = clients_clone.lock().unwrap();
                for (_, tx) in clients.iter() {
                    let _ = tx.send(Ok(message.clone()));
                }
            }

            // Dormir antes del siguiente ciclo
            sleep(Duration::from_secs(config_clone.monitoring_interval_sec)).await;
        }
    });

    // Iniciar servidor y esperar
    let host_str = config.host.clone();
    let port = config.port;
    
    warp::serve(ws_route).run((host_str.parse::<std::net::IpAddr>()?, port)).await;
    Ok(())
}
