use anyhow::{anyhow, Result};
use ethers::prelude::*;
use std::collections::HashMap;
use std::process::{Child, Command};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::config::Config;
use crate::models::{Fork, ForkStatus};

pub struct ForkManager {
    config: Config,
    forks: HashMap<Uuid, ForkInstance>,
    next_port: u16,
}

struct ForkInstance {
    fork: Fork,
    process: Child,
    provider: Arc<Provider<Http>>,
}

impl ForkManager {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            forks: HashMap::new(),
            next_port: 8545,
        }
    }

    pub async fn create_fork(&mut self, block_number: Option<u64>) -> Result<Fork> {
        if self.forks.len() >= self.config.max_forks {
            return Err(anyhow!("Maximum number of forks reached"));
        }

        let fork_id = Uuid::new_v4();
        let port = self.allocate_port();
        let rpc_url = format!("http://127.0.0.1:{}", port);

        info!("Creating fork {} on port {}", fork_id, port);

        // Build Anvil command
        let mut cmd = Command::new(&self.config.anvil_path);
        cmd.arg("--fork-url").arg(&self.config.fork_url);
        cmd.arg("--port").arg(port.to_string());
        cmd.arg("--no-mining");
        cmd.arg("--steps-tracing");

        if let Some(block) = block_number.or(self.config.fork_block_number) {
        // Start Anvil process
        let anvil = spawn(Anvil::default()).await?;

        // Wait for Anvil to be ready
        let provider = self.wait_for_fork(&rpc_url).await?;

        // Get fork info
        let block_number = provider.get_block_number().await?.as_u64();
        let chain_id = provider.get_chainid().await?.as_u64();

        let fork = Fork {
            id: fork_id,
            rpc_url: rpc_url.clone(),
            block_number,
            chain_id,
            status: ForkStatus::Active,
            created_at: chrono::Utc::now(),
        };

        let instance = ForkInstance {
            fork: fork.clone(),
            anvil,
            provider,
        };

        self.forks.insert(fork_id, instance);
        info!("Fork {} created successfully at block {}", fork_id, block_number);

        Ok(fork)
    }

    pub async fn destroy_fork(&mut self, fork_id: Uuid) -> Result<()> {
        if let Some(mut instance) = self.forks.remove(&fork_id) {
            info!("Destroying fork {}", fork_id);
            
            // Kill Anvil process
            if let Err(e) = instance.anvil.stop().await {
                warn!("Failed to kill Anvil process: {}", e);
            }
            
            Ok(())
        } else {
            Err(anyhow!("Fork not found"))
        }
    }

    pub fn get_fork(&self, fork_id: &Uuid) -> Option<&Fork> {
        self.forks.get(fork_id).map(|i| &i.fork)
    }

    pub fn get_provider(&self, fork_id: &Uuid) -> Option<Arc<Provider<Http>>> {
        self.forks.get(fork_id).map(|i| i.provider.clone())
    }

    pub fn list_forks(&self) -> Vec<Fork> {
        self.forks.values().map(|i| i.fork.clone()).collect()
    }

    async fn wait_for_fork(&self, rpc_url: &str) -> Result<Arc<Provider<Http>>> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(self.config.fork_timeout);

        loop {
            if start.elapsed() > timeout {
                return Err(anyhow!("Fork startup timeout"));
            }

            match Provider::<Http>::try_from(rpc_url) {
                Ok(provider) => {
                    // Try to get block number to verify it's ready
                    if provider.get_block_number().await.is_ok() {
                        debug!("Fork is ready at {}", rpc_url);
                        return Ok(Arc::new(provider));
                    }
                }
                Err(e) => {
                    debug!("Fork not ready yet: {}", e);
                }
            }

            sleep(Duration::from_millis(100)).await;
        }
    }

    fn allocate_port(&mut self) -> u16 {
        let port = self.next_port;
        self.next_port += 1;
        port
    }

    pub async fn cleanup_stale_forks(&mut self) {
        let stale_forks: Vec<Uuid> = self
            .forks
            .iter()
            .filter(|(_, instance)| {
                instance.fork.created_at < chrono::Utc::now() - chrono::Duration::hours(1)
            })
            .map(|(id, _)| *id)
            .collect();

        for fork_id in stale_forks {
            warn!("Cleaning up stale fork {}", fork_id);
            let _ = self.destroy_fork(fork_id).await;
        }
    }
}

impl Drop for ForkManager {
    fn drop(&mut self) {
        // Clean up all forks on shutdown
        for (fork_id, mut instance) in self.forks.drain() {
            error!("Cleaning up fork {} on shutdown", fork_id);
            let _ = instance.process.kill();
        }
    }
}



