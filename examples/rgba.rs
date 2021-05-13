use arrayfire::*;

fn main() {
    let dims = dim4!(4);
    let r = constant::<u8>(200, dims);
    let g = constant::<u8>(50, dims);
    let b = constant::<u8>(0, dims);
    let a = constant::<u8>(1, dims);

    let colors = join_many(1, vec![&r, &g, &b, &a]);
    af_print!("", flat(&transpose(&colors, false)));
}