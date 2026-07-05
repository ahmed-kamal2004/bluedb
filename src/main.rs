use config::config::Config;
use std::io::{Read, Result, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use tracing::info;

mod config;
mod engine;
use engine::engine::Engine;
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
    let engine = std::sync::Arc::new(Engine::new());

    let listener = TcpListener::bind(format!("{}:{}", config.host, config.port))?;

    for stream in listener.incoming() {
        let stream = stream?;

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

        match engine.process_query(&query) {
            Err(error) => {
                info!("Processing Error: {:?}", error);
            }
            _ => {}
        }

        let response = format!("Echo: {}", query);
        if let Err(e) = stream.write_all(response.as_bytes()) {
            eprintln!("Failed to write to stream: {}", e);
            return;
        }
    }
}
