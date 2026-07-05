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
    let engine = std::sync::Arc::new(Engine::new(config.clone()));

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
            // check startup validation first

            let mut buffer: [u8; 1024] = [0; 1024];
            let bytes_read = match stream.read(&mut buffer) {
                Ok(bytes_read) => bytes_read,
                Err(e) => {
                    eprintln!("Failed to read from stream: {}", e);
                    // close the connection
                    return;
                }
            };

            if bytes_read == 0 {
                eprintln!("No data received from stream");
                return;
            }

            let startup_message = String::from_utf8_lossy(&buffer[..bytes_read]);
            if !startup_message.starts_with("STARTUP") {
                eprintln!("Invalid startup message: {}", startup_message);
                return;
            }

            let database_name = startup_message.trim_start_matches("STARTUP").trim();
            info!("Client requested to connect to database: {}", database_name);

            // check if it exists
            if engine_cloned.contains_database(database_name) {
                info!(
                    "Database {} exists. Proceeding with connection.",
                    database_name
                );
            } else {
                eprintln!(
                    "Database {} does not exist. Closing connection.",
                    database_name
                );
                return;
            }

            connection_handler(stream, engine_cloned, database_name);
        });
    }

    Ok(())
}

fn connection_handler(mut stream: TcpStream, engine: Arc<Engine>, database_name: &str) {
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

        let response = match engine.process_query(&query, database_name) {
            Ok(result) => String::from_utf8_lossy(&result).to_string(),
            Err(e) => format!("ERROR: {:?}", e),
        };

        if response.is_empty() {
            let empty_response = "Query executed successfully, but no data to return.".to_string();
            if let Err(e) = stream.write_all(empty_response.as_bytes()) {
                eprintln!("Failed to write to stream: {}", e);
                return;
            }
        } else {
            if let Err(e) = stream.write_all(response.as_bytes()) {
                eprintln!("Failed to write to stream: {}", e);
                return;
            }
        }
    }
}
