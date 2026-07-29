use anyhow::Result;
use bluedb::result::QueryResult;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use serde_json;
use std::env;
use std::io::{Read, Write};

const CLIENT_BUFFER_SIZE: usize = 512;

fn main() -> Result<()> {
    let mut args = env::args();

    let target_db_server =
        env::var("BLUEDB_SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:5444".to_string());
    let mut stream =
        std::net::TcpStream::connect(&target_db_server).expect("Could not connect to server");

    let mut rl = DefaultEditor::new().expect("Failed to create rustyline editor");

    let source_port = stream.local_addr()?.port();
    let source_ip = stream.local_addr()?.ip();

    let mut buffer = String::new();
    loop {
        let prompt = if buffer.is_empty() {
            format!("testdb({}:{}) <-> ", source_ip, source_port)
        } else {
            "...> ".to_string()
        };

        let line = match rl.readline(&prompt) {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) => {
                buffer.clear();
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("exit");
                break;
            }
            Err(e) => {
                eprintln!("Error reading line: {:?}", e);
                break;
            }
        };

        if buffer.is_empty() && line.trim() == "exit" {
            stream
                .write_all(b"exit\n")
                .expect("Failed to send exit command to server");
            break;
        }

        buffer.push_str(&line);
        buffer.push('\n');

        if line.trim_end().ends_with(';') {
            stream
                .write_all(line.as_bytes())
                .expect("Failed to send data to server");

            let mut len_buf = [0u8; 8];
            stream
                .read_exact(&mut len_buf)
                .expect("Failed to read response length");
            let len = u64::from_be_bytes(len_buf) as usize;

            let mut chunk: [u8; CLIENT_BUFFER_SIZE] = [0; CLIENT_BUFFER_SIZE];
            let mut payload = Vec::new();
            while payload.len() < len {
                let to_read = std::cmp::min(len - payload.len(), CLIENT_BUFFER_SIZE);
                stream
                    .read_exact(&mut chunk[..to_read])
                    .expect("Failed to read response chunk");
                payload.extend_from_slice(&chunk[..to_read]);
            }

            let response: QueryResult =
                serde_json::from_slice(&payload).expect("Failed to parse response from server");

            println!("\n>--> {}", response);

            buffer.clear();
        }
    }

    Ok(())
}
