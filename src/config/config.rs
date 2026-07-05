pub struct Config {
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn new() -> Self {
        let bluedb_server_addr =
            std::env::var("BLUEDB_SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:5444".to_string());
        let parts: Vec<&str> = bluedb_server_addr.split(':').collect();
        let host = parts[0].to_string();
        let port = parts[1].parse::<u16>().unwrap_or(5444);

        Config { host, port }
    }
}
