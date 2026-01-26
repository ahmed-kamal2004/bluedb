use std::thread;
use std::time::Duration;

use buffer::pool::BufPool;
use buffer::request::{DiskRequest, Op};

fn main() {
    let mut pool = BufPool::initialize();
    // thread::sleep(Duration::from_secs(3));
    println!("{:?}", pool);
    println!("Inside pool");

    // let mut req = DiskRequest { operation:Op::READ, object_id: 1, page_id: 1};

    // // let out = pool.fetch_page(&req);

    // pool.add_file("/home/ahmed-kamal/Every/BlueDB/table001", 1);

    // println!(" {:?} ", pool);

    // let f1 = pool.get_file(1);
    // let f2 = pool.get_file(5);

    // println!(" {:?} {:?} ", f1, f2);

    // println!("Before change");
    // let p_Arc = pool.fetch_page(&req);
    // println!(" {:?} ", p_Arc);

 
    // println!("Making a change");
    // let mut page = p_Arc.write().unwrap();
    // let arr= &mut *page.page;
    // arr[0..4000].fill(1); 
    // drop(page);


    // println!("Writing");
    // req.operation = Op::WRITE;
    // pool.write_page(&req);


    // //fetch again
    // println!("Fetch after a change");
    // req.operation = Op::READ;
    // let p2_Arc = pool.fetch_page(&req);
    // let page_read = p2_Arc.read().unwrap();
    // println!(" {:?} ", page_read);

}
