#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ffi::c_void;

pub mod webgpu {
    include!(concat!(env!("OUT_DIR"), "/webgpu.rs"));
}

extern "C" {
    pub fn new_instance() -> *mut c_void;
    pub fn delete_instance(instance: *mut c_void);
    pub fn create_device(instance: *mut c_void) -> webgpu::WGPUDevice;
}
