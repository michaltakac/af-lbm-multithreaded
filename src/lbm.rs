use arrayfire::*;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::thread;

type FloatNum = f32;

fn normalize(a: &Array<FloatNum>) -> Array<FloatNum> {
    let min = min_all(a).0;
    let max = max_all(a).0;
    (a-min)/(max-min) as FloatNum
}

fn stream(f: &Array<FloatNum>) -> Array<FloatNum> {
  let mut pdf = f.clone();
  eval!(pdf[1:1:0, 1:1:0, 1:1:1] = shift(&view!(f[1:1:0, 1:1:0, 1:1:1]), &[1, 0, 0, 0]));
  eval!(pdf[1:1:0, 1:1:0, 2:2:1] = shift(&view!(f[1:1:0, 1:1:0, 2:2:1]), &[0, 1, 0, 0]));
  eval!(pdf[1:1:0, 1:1:0, 3:3:1] = shift(&view!(f[1:1:0, 1:1:0, 3:3:1]), &[-1, 0, 0, 0]));
  eval!(pdf[1:1:0, 1:1:0, 4:4:1] = shift(&view!(f[1:1:0, 1:1:0, 4:4:1]), &[0, -1, 0, 0]));
  eval!(pdf[1:1:0, 1:1:0, 5:5:1] = shift(&view!(f[1:1:0, 1:1:0, 5:5:1]), &[1, 1, 0, 0]));
  eval!(pdf[1:1:0, 1:1:0, 6:6:1] = shift(&view!(f[1:1:0, 1:1:0, 6:6:1]), &[-1, 1, 0, 0]));
  eval!(pdf[1:1:0, 1:1:0, 7:7:1] = shift(&view!(f[1:1:0, 1:1:0, 7:7:1]), &[-1, -1, 0, 0]));
  eval!(pdf[1:1:0, 1:1:0, 8:8:1] = shift(&view!(f[1:1:0, 1:1:0, 8:8:1]), &[1, -1, 0, 0]));
  pdf
}

pub fn lbm_d2q9(out: &mut Array<u8>){
  // Grid length, number and spacing
  let nx: u64 = 128;
  let ny: u64 = 128;

  let total_nodes = nx * ny;

  // Physical parameters.
  let ux_lid: FloatNum = 0.05; // horizontal lid velocity
  let uy_lid: FloatNum = 0.0; // vertical lid velocity
  let rho0: FloatNum = 1.0;

  // Reynolds number
  let re: FloatNum = 100.0;
  // Kinematic viscosity
  let nu: FloatNum = ux_lid * 2.0 * nx as FloatNum / re;
  // Relaxation time
  let tau: FloatNum = (3.0 as FloatNum) * nu + (0.5 as FloatNum);
  // Relaxation parameter
  let omega: FloatNum = (1.0 as FloatNum) / tau;

  let t1: FloatNum = 4. / 9.;
  let t2: FloatNum = 1. / 9.;
  let t3: FloatNum = 1. / 36.;

  let dims = dim4!(nx, ny);

  let ux_lid_af = constant::<FloatNum>(ux_lid, dims);
  let uy_lid_af = constant::<FloatNum>(uy_lid, dims);

  let lid = seq!(1, nx as i32 - 2, 1);
  let end_y = seq!(nx as i32 - 1, ny as i32 - 1, 1);

  //  c6  c2   c5
  //    \  |  /
  //  c3 -c0 - c1
  //    /  |  \
  //  c7  c4   c8
  // Discrete velocities
  let ex = Array::<FloatNum>::new(&[0., 1., 0., -1., 0., 1., -1., -1., 1.], dim4!(9));
  let ey = Array::<FloatNum>::new(&[0., 0., 1., 0., -1., 1., 1., -1., -1.], dim4!(9));

  // weights
  let w = Array::new(&[t1, t2, t2, t2, t2, t3, t3, t3, t3], dim4!(9));

  let ci: Array<u64> = (range::<u64>(dim4!(1, 8), 1) + 1) * total_nodes;
  let nbidx = Array::new(&[2, 3, 0, 1, 6, 7, 4, 5], dim4!(8));
  let span = seq!();
  let nbi: Array<u64> = view!(ci[span, nbidx]);

  let main_index = moddims(&range(dim4!(total_nodes * 9), 0), dim4!(nx, ny, 9));
  let nb_index = flat(&stream(&main_index));

  // Open lid
  let mut bound = constant::<FloatNum>(1.0, dims);
  let zeros = constant::<FloatNum>(0.0, dims);
  let all_except_top_lid = seq!(1, ny as i32 - 1, 1);
  assign_seq(
      &mut bound,
      &[lid, all_except_top_lid],
      &index(&zeros, &[lid, all_except_top_lid]),
  );

  // matrix offset of each Occupied Node
  let on = locate(&bound);

  // Bounceback indexes
  let to_reflect = flat(&tile(&on, dim4!(ci.elements() as u64)))
      + flat(&tile(&ci, dim4!(on.elements() as u64)));
  let reflected = flat(&tile(&on, dim4!(nbi.elements() as u64)))
      + flat(&tile(&nbi, dim4!(on.elements() as u64)));

  let mut density = constant::<FloatNum>(rho0, dims);
  let mut ux = constant::<FloatNum>(0.0, dims);
  let mut uy = constant::<FloatNum>(0.0, dims);

  let zeroed_on = constant::<FloatNum>(0.0, on.dims());

  // Start in equilibrium state
  let mut u_sq: Array<FloatNum> =
      flat(&(pow(&ux, &(2.0 as FloatNum), false) + pow(&uy, &(2.0 as FloatNum), false)));
  let mut eu: Array<FloatNum> = flat(
      &(&mul(&transpose(&ex, false), &flat(&ux), true)
          + &mul(&transpose(&ey, false), &flat(&uy), true)),
  );
  let mut f: Array<FloatNum> = flat(&mul(&transpose(&w, false), &flat(&density), true))
      * ((1.0 as FloatNum)
          + (3.0 as FloatNum) * &eu
          + (4.5 as FloatNum) * (&pow(&eu, &(2.0 as FloatNum), false))
          - (1.5 as FloatNum) * (&tile(&flat(&u_sq), dim4!(9))));

  let mut iter: u64 = 0;
  let maxiter: u64 = 3000;

  sync(0);
  println!("Simulation started...");

  while iter < maxiter {
      // Streaming by reading from neighbors (with pre-built index) - pull scheme
      let f_streamed = view!(f[nb_index]);

      let bouncedback = view!(f_streamed[to_reflect]); // Densities bouncing back at next timestep

      let f_2d = moddims(&f_streamed, dim4!(total_nodes, 9));

      // Compute macroscopic variables
      let rho = sum(&f_2d, 1);
      density = moddims(&rho, dims);

      let fex = mul(&transpose(&ex, false), &f_2d, true);
      let fey = mul(&transpose(&ey, false), &f_2d, true);

      ux = moddims(&(sum(&fex, 1) / &rho), dims);
      uy = moddims(&(sum(&fey, 1) / &rho), dims);

      // Macroscopic (Dirichlet) boundary conditions
      eval!(ux[lid, end_y] = view!(ux_lid_af[lid, end_y]));
      eval!(uy[lid, end_y] = view!(uy_lid_af[lid, end_y]));

      eval!(ux[on] = zeroed_on);
      eval!(uy[on] = zeroed_on);
      eval!(density[on] = zeroed_on);

      // Collision
      u_sq = flat(&(pow(&ux, &(2.0 as FloatNum), false) + pow(&uy, &(2.0 as FloatNum), false)));
      eu = flat(
          &(&mul(&transpose(&ex, false), &flat(&ux), true)
              + &mul(&transpose(&ey, false), &flat(&uy), true)),
      );
      let feq = flat(&mul(&transpose(&w, false), &flat(&density), true))
          * ((1.0 as FloatNum)
              + (3.0 as FloatNum) * &eu
              + (4.5 as FloatNum) * (&pow(&eu, &(2.0 as FloatNum), false))
              - (1.5 as FloatNum) * (&tile(&flat(&u_sq), dim4!(9))));

      f = omega * feq + (1.0 - omega) * f_streamed;

      eval!(f[reflected] = bouncedback);

      // Results
      let mut results = moddims(&sqrt(&u_sq), dims);
      eval!(results[on] = constant::<FloatNum>(FloatNum::NAN, on.dims()));
      results = flip(&transpose(&normalize(&results), false), 0);
      // Colormap for Unity's Texture2D (RGBA32)
      let r = flat(&((1.5f32-abs(&(1.0f32-4.0f32*(&results-0.5f32)))) * 255));
      let g = flat(&((1.5f32-abs(&(1.0f32-4.0f32*(&results-0.25f32)))) * 255));
      let b = flat(&((1.5f32-abs(&(1.0f32-4.0f32*&results))) * 255));
      let a = flat(&constant::<f32>(1.0 as FloatNum, dims));
      let mut colors = &flat(&transpose(&join_many(1, vec![&r, &g, &b, &a]), false)).cast::<u8>();
    //   out = colors.clone();
      // Copy to host (it's then read and copied into buffer in Unity)
    //   out = colors;

      sync(0);
      iter += 1;
  }
}