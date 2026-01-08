use buffer::pool::BufPool;
use buffer::request::{DiskRequest, Op};

fn main() {
    let mut pool = BufPool::new();

    let req = DiskRequest { operation:Op::READ, object_id: 0, page_id: 0 };

    let out = pool.fetch_page(&req);

}
