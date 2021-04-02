#![allow(dead_code)]
extern crate num_cpus;
extern crate sourcerenderer_core;
extern crate image;
extern crate crossbeam_channel;
extern crate crossbeam_utils;
extern crate sourcerenderer_bsp;
extern crate sourcerenderer_vpk;
extern crate sourcerenderer_vtf;
extern crate sourcerenderer_vmt;
extern crate sourcerenderer_mdl;
extern crate sourcerenderer_vvd;
extern crate sourcerenderer_vtx;
#[macro_use]
extern crate legion;
extern crate regex;
extern crate bitvec;
extern crate rayon;

#[cfg(feature = "threading")]
pub use self::engine::Engine;

pub use transform::Transform;
pub use transform::Parent;
pub use camera::Camera;
pub use camera::ActiveCamera;

#[cfg(feature = "threading")]
mod engine;

mod asset;
mod spinning_cube;
pub mod transform;
mod camera;
pub mod fps_camera;
mod math;

mod renderer;
mod game;
