use crate::config::Config;
use redis::aio::Connection;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db_pool: PgPool,
    pub redis_conn: Arc<RwLock<Connection>>,
}

impl AppState {
    pub fn new(
        config: Config,
        db_pool: PgPool,
        redis_conn: Arc<RwLock<Connection>>,
    ) -> Self {
        Self {
            config,
            db_pool,
            redis_conn,
        }
    }
}



