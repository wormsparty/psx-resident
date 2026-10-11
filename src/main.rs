#[cfg(not(target_arch = "wasm32"))]
mod native;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    native::run();
}
