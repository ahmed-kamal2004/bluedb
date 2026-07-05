#[derive(Debug)]
pub enum ProcessingError {
    ParseError(String),
}

#[derive(Debug)]
pub enum StartUpError {
    CatalogError(String),
}
