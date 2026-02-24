use buffer::builder::BufPoolBuilder;
use buffer::request::{BufferRequest, Op};
use disk::page::PageKey;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub mod catalog;
pub mod storage;

fn main() {
    let mut pool = Arc::new(
        BufPoolBuilder::default()
            .set_file_info("/home/ahmed-kamal/Every/BlueDB/table-info".to_string())
            .set_directory_data("/home/ahmed-kamal/Every/BlueDB/data".to_string())
            .set_eviction_period(1)
            .set_buffer_capacity(1)
            .set_eviction_threshold(0)
            .build(),
    );

    // thread::sleep(Duration::from_secs(3));

    let mut req = BufferRequest {
        operation: Op::READ,
        page_key: PageKey::new(1, 1),
    };

    let lock_guard = { pool.acquire_page(&req).unwrap() };

    thread::sleep(Duration::from_secs(2));
    let arc_pool = Arc::clone(&pool);
    let join_th1 = thread::spawn(move || {
        let req2 = BufferRequest {
            operation: Op::READ,
            page_key: PageKey::new(1, 1),
        };
        let lock_guard2 = { arc_pool.acquire_page(&req2).unwrap() };
        thread::sleep(Duration::from_secs(10));
        arc_pool.release_page(lock_guard2);
    });

    // join_th1.join().unwrap();
    thread::sleep(Duration::from_secs(2));

    {
        pool.release_page(lock_guard);
    }

    req.operation = Op::WRITE;

    let lock_guard2 = { pool.acquire_page(&req).unwrap() };

    thread::sleep(Duration::from_secs(10));

    {
        pool.release_page(lock_guard2);
    }

    thread::sleep(Duration::from_secs(5));

    let arc_pool2 = Arc::clone(&pool);
    let join_th2 = thread::spawn(move || {
        let req3 = BufferRequest {
            operation: Op::WRITE,
            page_key: PageKey::new(1, 1),
        };
        let lock_guard3 = { arc_pool2.acquire_page(&req3).unwrap() };
        thread::sleep(Duration::from_secs(10));
        arc_pool2.release_page(lock_guard3);
    });

    let arc_pool3 = Arc::clone(&pool);
    let join_th3 = thread::spawn(move || {
        let req3 = BufferRequest {
            operation: Op::WRITE,
            page_key: PageKey::new(1, 1),
        };
        let lock_guard3 = { arc_pool3.acquire_page(&req3).unwrap() };
        thread::sleep(Duration::from_secs(10));
        arc_pool3.release_page(lock_guard3);
    });

    let arc_pool4 = Arc::clone(&pool);
    let join_th4 = thread::spawn(move || {
        let req2 = BufferRequest {
            operation: Op::READ,
            page_key: PageKey::new(1, 1),
        };
        let lock_guard2 = { arc_pool4.acquire_page(&req2).unwrap() };
        thread::sleep(Duration::from_secs(1));
        arc_pool4.release_page(lock_guard2);
    });

    join_th2.join().unwrap();
    join_th3.join().unwrap();
    join_th4.join().unwrap();
}
