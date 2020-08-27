#[macro_use]
extern crate bitflags;
extern crate crossbeam_channel;
extern crate crossbeam_queue;

pub mod graphics;
pub mod platform;
pub mod pool;

pub use crate::platform::Platform;

pub type Vec2 = vek::Vec2<f32>;
pub type Vec3 = vek::Vec3<f32>;
pub type Vec4 = vek::Vec4<f32>;
pub type Vec2I = vek::Vec2<i32>;
pub type Vec2UI = vek::Vec2<u32>;
