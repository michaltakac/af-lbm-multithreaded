use arrayfire::*;

struct Sim {
    results: Vec<u8>,
    ptr: *mut u8,
    colors: Array<u8>
}

impl Sim {
    fn new() -> Sim {
        let dims = dim4!(2);
        let colors = constant::<u8>(1u8, dims);
        let mut results = vec!(0u8; 2);
        let ptr = results.as_mut_ptr();
        Sim { colors, results, ptr }
    }
    fn sim(&mut self){
        for _ in 0..10 {
            let a = randu::<u8>(self.colors.dims());
            self.colors = a;
        }
    }
    fn data(&mut self) {
        self.colors.host(self.results.as_mut_slice());
    }
    fn inspect(&self) {
        println!("Result: {:?}, ptr: {:?}", self.results, self.ptr);
    }
}

fn main() {
    let mut sim = Sim::new();
    sim.inspect();
    sim.sim();
    sim.inspect();
    sim.data();
    sim.inspect();
}