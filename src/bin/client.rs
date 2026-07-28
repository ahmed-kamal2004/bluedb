use anyhow::Result;
use bluedb::result::QueryResult;
use rustyline::DefaultEditor;
use serde_json;
use std::env;
use std::io::{Read, Write};
fn main() -> Result<()> {
    let mut args = env::args();

    let target_db_server =
        env::var("BLUEDB_SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:5444".to_string());
    let mut stream =
        std::net::TcpStream::connect(&target_db_server).expect("Could not connect to server");

    let mut rl = DefaultEditor::new().expect("Failed to create rustyline editor");

    let source_port = stream.local_addr()?.port();
    let source_ip = stream.local_addr()?.ip();

    let prompt = format!("testdb({}:{}) <-> ", source_ip, source_port);

    loop {
        let line = rl.readline(&prompt).expect("Failed to read line");

        if line.trim() == "exit" {
            break;
        }

        stream
            .write_all(line.as_bytes())
            .expect("Failed to send data to server");

        let mut buffer = [0; 4096];
        let mut bytes_read = stream
            .read(&mut buffer)
            .expect("Failed to read response from server");
        let mut total_response = Vec::new();
        total_response.extend_from_slice(&buffer[..bytes_read]);
        while bytes_read != 0 && buffer[bytes_read - 1] != b'\n' {
            let bytes_read = stream
                .read(&mut buffer)
                .expect("Failed to read additional response from server");
            total_response.extend_from_slice(&buffer[..bytes_read]);
        }

        let response: QueryResult =
            serde_json::from_slice(&total_response).expect("Failed to parse response from server");

        println!("\n\tResponse <> \n{}", response);
    }

    Ok(())
}
