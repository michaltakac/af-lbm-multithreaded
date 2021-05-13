use arrayfire::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock, mpsc};
use std::thread;
use std::time::Duration;

struct Sim {
    results: Vec<u8>,
    ptr: *mut u8,
    dims: Dim4,
    colors: Arc<RwLock<Array<u8>>>
}


impl Sim {
    fn new() -> Sim {
        let dims = dim4!(1e7 as u64);
        let colors = constant::<u8>(1u8, dims);
        let mut results = vec![0u8; 1e7 as usize];
        let ptr = results.as_mut_ptr();

        let colors_lock = Arc::new(RwLock::new(colors));

        Sim {
            colors: colors_lock,
            dims,
            results,
            ptr
        }
    }
    pub fn sim(&mut self) {
        let dims = self.dims.clone();
        let wlock = self.colors.clone();

        Some(thread::spawn(move || {
            set_device(0);
            let mut b = constant::<u8>(1u8, dims);
            for i in 0..500000 {
                let x = randu::<u8>(dims);
                b = x;
                sync(0);
                if i % 10 == 0 {
                    if let Ok(mut c_guard) = wlock.write() {
                        *c_guard = b;
                    }
                }
            }
        }));
    }
    fn data(&mut self) {
        let read_guard = self.colors.read().unwrap();
        read_guard.host(self.results.as_mut_slice());
    }
    fn inspect(&self) {
        println!("Result: {:?}, ptr: {:?}", self.results[0], self.ptr);
    }
}

fn main() {
    info();

    let mut sim = Sim::new();
    sim.inspect();
    sim.sim();
    mem_info!("Initial memory");
    sim.inspect();
    sim.data();
    mem_info!("moved memory");

    for _ in 0..10 {
        sim.data();
        sim.inspect();
        thread::sleep(Duration::from_secs(1));
        mem_info!("Processing memory");
    }
}
