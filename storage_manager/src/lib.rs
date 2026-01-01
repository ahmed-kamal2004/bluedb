use crate::tlv::TLV;

pub(crate) mod fmanager;
pub(crate) mod schema;
pub(crate) mod tlv;

pub fn create_random_tlv() -> tlv::TLV {
    TLV::new()
}

pub fn get_serialized(tlv: &TLV) -> Vec<u8> {
    tlv.serialize()
}

// pub create_schema {

// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // assert_eq!(result, 4);
    }
}
