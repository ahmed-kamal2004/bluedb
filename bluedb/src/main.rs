use buffer::page::PageKey;
use buffer::pool::BufPool;
use buffer::request::{DiskRequest, Op};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn main() {
    let pool = Arc::new(BufPool::initialize());

    // thread::sleep(Duration::from_secs(3));
    println!("{:?}", pool);
    println!("Inside pool");

    let mut req = DiskRequest {
        operation: Op::READ,
        page_key: PageKey::new(1, 1),
    };

    let lock_guard = { pool.acquire_page(&req).unwrap() };

    thread::sleep(Duration::from_secs(2));
    let arc_pool = Arc::clone(&pool);
    let join_th1 = thread::spawn(move || {
        let req2 = DiskRequest {
            operation: Op::READ,
            page_key: PageKey::new(1, 1),
        };
        let lock_guard2 = { arc_pool.acquire_page(&req2).unwrap() };
        thread::sleep(Duration::from_secs(2));
        arc_pool.release_page(lock_guard2);
    });

    join_th1.join().unwrap();
    thread::sleep(Duration::from_secs(2));

    {
        pool.release_page(lock_guard);
    }

    req.operation = Op::WRITE;

    let lock_guard2 = { pool.acquire_page(&req).unwrap() };

    thread::sleep(Duration::from_secs(2));

    {
        pool.release_page(lock_guard2);
    }

    thread::sleep(Duration::from_secs(5));

    let arc_pool2 = Arc::clone(&pool);
    let join_th2 = thread::spawn(move || {
        let req3 = DiskRequest {
            operation: Op::WRITE,
            page_key: PageKey::new(1, 1),
        };
        let lock_guard3 = { arc_pool2.acquire_page(&req3).unwrap() };
        thread::sleep(Duration::from_secs(2));
        arc_pool2.release_page(lock_guard3);
    });

    let arc_pool3 = Arc::clone(&pool);
    let join_th3 = thread::spawn(move || {
        let req3 = DiskRequest {
            operation: Op::WRITE,
            page_key: PageKey::new(1, 1),
        };
        let lock_guard3 = { arc_pool3.acquire_page(&req3).unwrap() };
        thread::sleep(Duration::from_secs(2));
        arc_pool3.release_page(lock_guard3);
    });

    let arc_pool4 = Arc::clone(&pool);
    let join_th4 = thread::spawn(move || {
        let req2 = DiskRequest {
            operation: Op::READ,
            page_key: PageKey::new(1, 1),
        };
        let lock_guard2 = { arc_pool4.acquire_page(&req2).unwrap() };
        thread::sleep(Duration::from_secs(2));
        arc_pool4.release_page(lock_guard2);
    });

    join_th2.join().unwrap();
    join_th3.join().unwrap();
    join_th4.join().unwrap();
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
