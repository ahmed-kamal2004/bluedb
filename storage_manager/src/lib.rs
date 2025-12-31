use crate::tlv::TLV;

pub (crate) mod tlv;


pub fn add(left: u64, right: u64) -> u64 {
    left + right
}


pub fn create_random_tlv() -> tlv::TLV {
    TLV::new()
}

pub fn get_serialized(tlv: &TLV) -> Vec<u8> {
    tlv.serialize()
} 



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
