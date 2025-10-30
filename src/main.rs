#[cfg(target_arch = "wasm32")]
use raytracing::run_web;

#[cfg(not(target_arch = "wasm32"))]
use raytracing::run;

fn main() {
    #[cfg(target_arch = "wasm32")]
    run_web().unwrap();
    #[cfg(not(target_arch = "wasm32"))]
    run().unwrap();
}
