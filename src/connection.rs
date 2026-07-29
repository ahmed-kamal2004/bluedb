use crate::engine::engine::Engine;
use crate::result::QueryResult;
use anyhow::Result;
use std::io::Read;
use std::{io::Write, net::TcpStream, sync::Arc};
use tracing::{error, info};

/// Buffer Read size
const BUFFER_SIZE: usize = 512;

/// Connection struct
/// Contains the transaction, connection stream
pub struct Connection {
    pub conn_id: u64,
    stream: TcpStream,
}

impl Connection {
    pub fn new(stream: TcpStream, conn_id: u64) -> Result<Self> {
        // stream.set_read_timeout(Some(READ_TIMEOUT))?;

        Ok(Connection { stream, conn_id })
    }

    pub fn handle(&mut self, engine: Arc<Engine>) -> Result<()> {
        let mut buffer: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
        let mut query = String::new();
        loop {
            let bytes_read = match self.stream.read(&mut buffer) {
                Ok(result) => result,
                Err(e) => {
                    error!("Failed to read from stream: {}", e);
                    return Err(anyhow::anyhow!("Failed to read from stream: {}", e));
                }
            };

            if bytes_read == 0 {
                // we can't subtract 1 from 0, so we check this first
                info!("Connection: Client closed the connection.");
                break;
            }

            query.push_str(&String::from_utf8_lossy(&buffer[..bytes_read]));
            if buffer[bytes_read - 1] == b';' {
                // the client is terminating the query, now time to execute it
                let response = self.execute_query(engine.clone(), &query)?;

                self.send_response(&response)?;

                query.clear(); // clear the query buffer for the next query
            } else if (query.len() == 5 && query == "exit\n")
                || (query.len() == 6 && query == "exit;\n")
                || (query.len() == 3 && query == "\\q\n")
            {
                info!("Connection: Client requested to exit.");
                break;
            } else {
                continue; // just contniue reading the query packets.
            }
        }
        Ok(())
    }

    fn execute_query(&mut self, engine: Arc<Engine>, query: &str) -> Result<QueryResult> {
        match engine.process_query(query, self.conn_id) {
            Ok(result) => Ok(result),
            Err(e) => {
                let error_result = QueryResult::Error {
                    message: format!("Error processing query: {}", e),
                };
                Ok(error_result)
            }
        }
    }

    fn send_response(&mut self, response: &QueryResult) -> Result<()> {
        let response_bytes = serde_json::to_vec(response)?;
        let len = response_bytes.len();
        self.stream.write_all(&len.to_be_bytes())?;
        self.stream.write_all(&response_bytes)?;
        Ok(())
    }
}
