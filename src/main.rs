use anyhow::Result;
use bluedb::config::config::Config;
use bluedb::engine::engine::Engine;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use tracing::{error, info};

fn main() -> Result<()> {
    // initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // get config
    let config = Config::new();

    // initialize Engine.
    let mut engine = Engine::new(config.clone());
    match engine.initialize() {
        Ok(_) => info!("Engine initialized successfully."),
        Err(e) => {
            error!("Failed to initialize Engine: {:?}", e);
            return Err(anyhow::anyhow!("Engine initialization failed"));
        }
    }
    let engine = Arc::new(engine);

    let listener = TcpListener::bind(format!("{}:{}", config.host, config.port))?;

    for stream in listener.incoming() {
        let mut stream = stream?;

        let peer_addr = stream.peer_addr()?;
        let peer_port = peer_addr.port();
        let peer_ip = peer_addr.ip();

        info!(
            "Got new connection Request from source {}:{}",
            peer_ip, peer_port
        );

        let engine_cloned = engine.clone();

        thread::spawn(move || {
            connection_handler(stream, engine_cloned);
        });
    }

    Ok(())
}

fn connection_handler(mut stream: TcpStream, engine: Arc<Engine>) {
    let mut buffer: [u8; 4096] = [0; 4096];
    loop {
        let bytes_read = match stream.read(&mut buffer) {
            Ok(bytes_read) => bytes_read,
            Err(e) => {
                eprintln!("Failed to read from stream: {}", e);
                return;
            }
        };

        if bytes_read == 0 {
            break;
        }

        let query = String::from_utf8_lossy(&buffer[..bytes_read]);

        let response = match engine.process_query(&query) {
            Ok(result) => serde_json::to_vec(&result),
            Err(e) => serde_json::to_vec(&format!("Error processing query: {:?}", e)),
        };

        if let Ok(response) = response {
            if response.is_empty() {
                let empty_response =
                    "Query executed successfully, but no data to return.".to_string();
                if let Err(e) = stream.write_all(empty_response.as_bytes()) {
                    eprintln!("Failed to write to stream: {}", e);
                    return;
                }
            } else {
                if let Err(e) = stream.write_all(&response) {
                    eprintln!("Failed to write to stream: {}", e);
                    return;
                }
            }
        } else {
            eprintln!("Failed to serialize response");
            return;
        }
    }
}
