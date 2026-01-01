#[derive(Copy, Clone, Debug)]
enum Type {
    INT,
    CHAR,
}

#[derive(Clone, Debug)]
pub struct TLV {
    ftype: Type,
    fvalue: Vec<u8>,
    flen: u8,
}

impl TLV {
    pub fn new() -> TLV {
        TLV {
            ftype: Type::CHAR,
            fvalue: vec![0, 0],
            flen: 4,
        }
    }

    pub fn serialize(self: &Self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.flen.to_be_bytes());
        bytes.extend_from_slice(&(self.ftype as u8).to_be_bytes());
        for val in &self.fvalue {
            bytes.extend_from_slice(&(*val as u8).to_be_bytes());
        }
        bytes
    }
}
