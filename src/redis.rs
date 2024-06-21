use fred::{
    prelude::*,
    types::{PerformanceConfig, RedisConfig},
};
use std::sync::OnceLock;
use tracing::{debug, info};

static REDISPOOL: OnceLock<RedisPool> = OnceLock::<RedisPool>::new();

pub fn init_redis_pool(
    redis_host: String,
    redis_port: u16,
    redis_pass: String,
) -> &'static RedisPool {
    REDISPOOL.get_or_init(|| {
        let redis_config = RedisConfig {
            server: ServerConfig::new_centralized(&redis_host, redis_port),
            password: Some(redis_pass),
            ..Default::default()
        };
        let performance = PerformanceConfig {
            default_command_timeout: std::time::Duration::from_millis(1000 * 3),
            ..Default::default()
        };
        let policy = ReconnectPolicy::new_linear(5, 1000 * 5, 100);
        let connection_config = ConnectionConfig::default();

        debug!("redis config: {:?}.", &redis_config);
        let redis_pool = RedisPool::new(
            redis_config,
            Some(performance),
            Some(connection_config),
            Some(policy),
            5,
        )
        .unwrap();
        let _join_handler = redis_pool.connect();
        futures::executor::block_on(async {
            redis_pool.wait_for_connect().await.unwrap();
        });
        info!("connect to redis successfully");
        redis_pool
    })
}

pub fn get_redis_pool() -> &'static RedisPool {
    REDISPOOL.get().unwrap()
}
