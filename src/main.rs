use anyhow::Result;
use bluedb::config::config::Config;
use bluedb::connection::Connection;
use bluedb::engine::engine::Engine;
use bluedb::initializer::Initializer;
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;
use tracing::{error, info};

static CONNECTION_ID_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn main() -> Result<()> {
    // initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // initialize the config
    let config = Config::new();

    // initialize the system (create necessary files and directories)
    Initializer::initialize_system(&config.main_db_path)?;

    // initialize the engine
    let engine = Arc::new(Engine::new(config.clone())?);

    // start listening for incoming connections
    let listener = TcpListener::bind(format!("{}:{}", config.host, config.port))?;
    info!("Server listening on {}:{}", config.host, config.port);

    for stream in listener.incoming() {
        let stream = stream?;
        let engine_cloned = engine.clone();
        // create a new connection to handle requests, responses and transactions

        let mut connection = Connection::new(
            stream,
            CONNECTION_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        )?;
        thread::spawn(move || {
            info!("Handling new connection with ID: {}", connection.conn_id);
            connection.handle(engine_cloned).unwrap_or_else(|e| {
                error!("Error handling connection: {:?}", e);
            });
        });
    }

    Ok(())
}
