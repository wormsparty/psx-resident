//! « Le Manoir des Marais » : un survival horror façon PlayStation 1,
//! entièrement en Rust avec un rasteriseur logiciel. Compile en natif
//! (winit + softbuffer) et en WebAssembly (canvas 2D).

pub mod actors;
pub mod audio;
pub mod game;
pub mod input;
pub mod math;
pub mod mesh;
pub mod options;
pub mod outdoor;
pub mod raster;
pub mod textures;
pub mod world;

#[cfg(target_arch = "wasm32")]
mod web;
