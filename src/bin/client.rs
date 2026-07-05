use rustyline::DefaultEditor;
use std::env;
use std::io::{Read, Result, Write};

fn main() -> Result<()> {
    let mut args = env::args();

    let _program = args.next().unwrap();
    let database = args.next().unwrap();

    let target_db_server =
        env::var("BLUEDB_SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:5444".to_string());
    let mut stream =
        std::net::TcpStream::connect(&target_db_server).expect("Could not connect to server");

    initiate_connection(&mut stream, &database)?;

    let mut rl = DefaultEditor::new().expect("Failed to create rustyline editor");

    let source_port = stream.local_addr()?.port();
    let source_ip = stream.local_addr()?.ip();

    let prompt = format!("{}({}:{}) <-> ", database, source_ip, source_port);

    loop {
        let line = rl.readline(&prompt).expect("Failed to read line");

        if line.trim() == "exit" {
            break;
        }

        stream
            .write_all(line.as_bytes())
            .expect("Failed to send data to server");

        let mut buffer = [0; 4096];
        let bytes_read = stream
            .read(&mut buffer)
            .expect("Failed to read from server");

        let response = String::from_utf8_lossy(&buffer[..bytes_read]);

        println!("\n\tResponse <> \n{}", response);
    }

    Ok(())
}

fn initiate_connection(mut stream: &std::net::TcpStream, database: &str) -> Result<()> {
    let startup_message = format!("STARTUP {}", database);
    stream.write_all(startup_message.as_bytes())?;
    Ok(())
}
