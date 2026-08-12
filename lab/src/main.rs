use memmap2::{Mmap, MmapMut};
use minifb::{Key, Window, WindowOptions};
use rand::Rng;
use std::env;
use std::ffi::CString;
use std::os::unix::io::FromRawFd;
use std::slice;
use std::thread::sleep;
use std::time::Duration;

const WIDTH: usize = 400;
const HEIGHT: usize = 400;
const TOTAL_PIXELS: usize = WIDTH * HEIGHT;
const SHM_PATH: &str = "/mmap_grayscale_pixels";

// --- TESTING TUNING KNOB ---
const NOISE_INTERVAL: Duration = Duration::from_millis(100);

fn main() {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match mode {
        "writer" => run_writer(),
        "reader" => run_reader(),
        _ => {
            println!("Usage:");
            println!("  cargo run -- writer  # Generates byte noise (0-255)");
            println!("  cargo run -- reader  # Visualizes full grayscale spectrum");
        }
    }
}

/// The Writer process: generates raw u8 values anywhere from 0 to 255
fn run_writer() {
    let shm_name = CString::new(SHM_PATH).unwrap();

    unsafe {
        let fd = libc::shm_open(shm_name.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o0666);
        assert!(fd >= 0, "Failed to create shared memory block");
        
        libc::ftruncate(fd, TOTAL_PIXELS as libc::off_t);

        let file = std::fs::File::from_raw_fd(fd);
        let mut mmap = MmapMut::map_mut(&file).expect("Failed to mmap");
        let memory_bytes: &mut [u8] = slice::from_raw_parts_mut(mmap.as_mut_ptr(), TOTAL_PIXELS);

        // Initialize to middle gray (128)
        for byte in memory_bytes.iter_mut() {
            *byte = 128;
        }

        println!("Memory initialized to 128 (Gray).");
        println!("Firing full-spectrum u8 noise every {:?}", NOISE_INTERVAL);

        let mut seed = 123456789u32;

        loop {
            for _ in 0..1 {
                seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
                let target_index = (seed as usize) % TOTAL_PIXELS;
                let target_end = (target_index + 4000).min(TOTAL_PIXELS);

                // Generate any raw byte value between 0 and 255
                let rnd_byte: u8 = rand::thread_rng().r#gen::<u8>();

                for i in target_index..target_end {
                    memory_bytes[i] = rnd_byte;
                }
            }

            sleep(NOISE_INTERVAL);
        }
    }
}

/// The Reader process: converts raw u8 values directly to matching pixel intensities
fn run_reader() {
    let shm_name = CString::new(SHM_PATH).unwrap();

    unsafe {
        let fd = libc::shm_open(shm_name.as_ptr(), libc::O_RDONLY, 0o0666);
        if fd < 0 {
            eprintln!("Error: Shared memory not found. Run the writer first.");
            return;
        }

        let file = std::fs::File::from_raw_fd(fd);
        let mmap = Mmap::map(&file).expect("Failed to map shared memory");
        let shared_bytes: &[u8] = slice::from_raw_parts(mmap.as_ptr(), TOTAL_PIXELS);

        let mut window = Window::new("Grayscale Byte Inspector", WIDTH, HEIGHT, WindowOptions::default())
            .unwrap();
        window.limit_update_rate(Some(Duration::from_micros(16600)));

        let mut screen_buffer = vec![0u32; TOTAL_PIXELS];

        println!("Connected. Visualizing raw byte states as grayscale colors...");

        while window.is_open() && !window.is_key_down(Key::Escape) {
            for i in 0..TOTAL_PIXELS {
                let intensity = shared_bytes[i] as u32;
                
                // Map the single byte value across Red, Green, and Blue channels
                // This produces perfect grayscale matching (0x00RRGGBB)
                screen_buffer[i] = (intensity << 16) | (intensity << 8) | intensity;
            }

            window.update_with_buffer(&screen_buffer, WIDTH, HEIGHT).unwrap();
        }
    }
}
