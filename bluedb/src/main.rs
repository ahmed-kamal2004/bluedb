use storage_manager::{add, create_random_tlv, get_serialized};

fn main() {
    let tlv = create_random_tlv();
    let bytes = get_serialized(&tlv);
    println!(" {:?} ", bytes);
}
