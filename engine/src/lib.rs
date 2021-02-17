#![allow(dead_code)]
extern crate num_cpus;
extern crate sourcerenderer_core;
extern crate sourcerenderer_vulkan;
extern crate async_std;
extern crate image;
extern crate crossbeam_channel;
extern crate crossbeam_utils;
extern crate sourcerenderer_bsp;
extern crate sourcerenderer_vpk;
extern crate sourcerenderer_vtf;
extern crate sourcerenderer_vmt;
#[macro_use]
extern crate legion;
extern crate regex;
extern crate bitvec;

pub use self::engine::Engine;
pub use transform::Transform;
pub use transform::Parent;
pub use camera::Camera;
pub use camera::ActiveCamera;

mod engine;
mod asset;
mod spinning_cube;
mod transform;
mod camera;
pub mod fps_camera;

mod renderer;
mod scene;
