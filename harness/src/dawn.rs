use std::ffi::{c_void, CStr, CString};
use std::mem::zeroed;
use std::ptr::{null, null_mut};

use color_eyre::Result;
use dawn::webgpu::*;
use futures::channel::oneshot;

use crate::Buffer;

struct Instance(*mut c_void);

impl Instance {
    pub fn new() -> Instance {
        Instance(unsafe { dawn::new_instance() })
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            dawn::delete_instance(self.0);
        }
    }
}

struct Device {
    _instance: Instance,
    handle: *mut dawn::webgpu::WGPUDeviceImpl,
}

impl Device {
    pub fn create() -> (Device, DeviceQueue) {
        let instance = Instance::new();
        let handle = unsafe { dawn::create_device(instance.0) };

        let device = Device {
            _instance: instance,
            handle,
        };

        let queue = DeviceQueue {
            handle: unsafe { wgpuDeviceGetQueue(handle) },
        };

        (device, queue)
    }

    pub fn create_shader_module(&self, source: &str) -> ShaderModule {
        let source = CString::new(source).unwrap();
        unsafe {
            let wgsl_descriptor = WGPUShaderModuleWGSLDescriptor {
                chain: WGPUChainedStruct {
                    sType: WGPUSType_WGPUSType_ShaderModuleWGSLDescriptor,
                    ..zeroed()
                },
                source: source.as_ptr() as _,
            };

            let descriptor = WGPUShaderModuleDescriptor {
                nextInChain: &wgsl_descriptor as *const _ as _,
                ..zeroed()
            };

            ShaderModule {
                handle: wgpuDeviceCreateShaderModule(self.handle, &descriptor),
            }
        }
    }

    pub fn create_compute_pipeline(
        &self,
        shader_module: &ShaderModule,
        entrypoint: &str,
    ) -> ComputePipeline {
        let entrypoint = CString::new(entrypoint).unwrap();
        unsafe {
            ComputePipeline {
                handle: wgpuDeviceCreateComputePipeline(
                    self.handle,
                    &WGPUComputePipelineDescriptor {
                        label: null(),
                        nextInChain: null(),
                        layout: null_mut(),
                        compute: WGPUProgrammableStageDescriptor {
                            constantCount: 0,
                            constants: null(),
                            module: shader_module.handle,
                            entryPoint: entrypoint.as_ptr(),
                            nextInChain: null(),
                        },
                    },
                ),
            }
        }
    }

    pub fn create_buffer(
        &self,
        mapped: bool,
        size: usize,
        usage: DeviceBufferUsage,
    ) -> DeviceBuffer {
        unsafe {
            DeviceBuffer {
                handle: wgpuDeviceCreateBuffer(
                    self.handle,
                    &WGPUBufferDescriptor {
                        label: null(),
                        nextInChain: null(),
                        mappedAtCreation: mapped,
                        size: size as _,
                        usage: usage.bits as _,
                    },
                ),
            }
        }
    }

    pub fn create_bind_group(
        &self,
        layout: &BindGroupLayout,
        entries: &[BindGroupEntry],
    ) -> BindGroup {
        unsafe {
            let entries = entries.iter().map(|e| e.into()).collect::<Vec<_>>();
            BindGroup {
                handle: wgpuDeviceCreateBindGroup(
                    self.handle,
                    &WGPUBindGroupDescriptor {
                        nextInChain: null(),
                        label: null(),
                        layout: layout.handle,
                        entries: entries.as_ptr(),
                        entryCount: entries.len() as _,
                    },
                ),
            }
        }
    }

    pub fn create_command_encoder(&self) -> CommandEncoder {
        unsafe {
            CommandEncoder {
                handle: wgpuDeviceCreateCommandEncoder(self.handle, &zeroed()),
            }
        }
    }

    pub fn tick(&self) {
        unsafe {
            wgpuDeviceTick(self.handle);
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            wgpuDeviceRelease(self.handle);
        }
    }
}

struct DeviceQueue {
    handle: WGPUQueue,
}

impl DeviceQueue {
    pub fn submit(&self, commands: &CommandBuffer) {
        unsafe {
            wgpuQueueSubmit(self.handle, 1, &commands.handle);
        }
    }
}

impl Drop for DeviceQueue {
    fn drop(&mut self) {
        unsafe {
            wgpuQueueRelease(self.handle);
        }
    }
}

struct ShaderModule {
    handle: WGPUShaderModule,
}

struct ShaderModuleCompilationInfo {
    pub success: bool,
    pub messages: Vec<ShaderModuleCompilationMessage>,
}

struct ShaderModuleCompilationMessage {
    pub line_number: u64,
    pub line_offset: u64,
    pub message: String,
}

impl ShaderModule {
    pub async fn get_compilation_info(&self) -> ShaderModuleCompilationInfo {
        let (tx, rx) =
            oneshot::channel::<(WGPUCompilationInfoRequestStatus, *const WGPUCompilationInfo)>();
        let mut tx = Some(tx);

        unsafe extern "C" fn compilation_callback(
            status: WGPUCompilationInfoRequestStatus,
            info: *const WGPUCompilationInfo,
            userdata: *mut c_void,
        ) {
            let tx = userdata
                as *mut Option<
                    oneshot::Sender<(WGPUCompilationInfoRequestStatus, *const WGPUCompilationInfo)>,
                >;
            (*tx).take().unwrap().send((status, info)).unwrap();
        }

        let mut messages = vec![];
        unsafe {
            wgpuShaderModuleGetCompilationInfo(
                self.handle,
                Some(compilation_callback),
                &mut tx as *mut _ as _,
            );

            let (status, info) = rx.await.unwrap();
            for i in 0..(*info).messageCount {
                let message = (*info).messages.offset(i as _);
                let str = CStr::from_ptr((*message).message);

                messages.push(ShaderModuleCompilationMessage {
                    line_number: (*message).lineNum,
                    line_offset: (*message).linePos,
                    message: str.to_str().unwrap().to_owned(),
                });
            }

            ShaderModuleCompilationInfo {
                success: status
                    == WGPUCompilationInfoRequestStatus_WGPUCompilationInfoRequestStatus_Success,
                messages,
            }
        }
    }
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        unsafe {
            wgpuShaderModuleRelease(self.handle);
        }
    }
}

struct ComputePipeline {
    handle: WGPUComputePipeline,
}

impl ComputePipeline {
    pub fn get_bind_group_layout(&self, index: u32) -> BindGroupLayout {
        unsafe {
            BindGroupLayout {
                handle: wgpuComputePipelineGetBindGroupLayout(self.handle, index),
            }
        }
    }
}

impl Drop for ComputePipeline {
    fn drop(&mut self) {
        unsafe {
            wgpuComputePipelineRelease(self.handle);
        }
    }
}

struct DeviceBuffer {
    handle: WGPUBuffer,
}

bitflags::bitflags! {
    struct DeviceBufferUsage: WGPUBufferUsage {
        const STORAGE = WGPUBufferUsage_WGPUBufferUsage_Storage;
        const COPY_SRC = WGPUBufferUsage_WGPUBufferUsage_CopySrc;
        const COPY_DST = WGPUBufferUsage_WGPUBufferUsage_CopyDst;
        const MAP_READ = WGPUBufferUsage_WGPUBufferUsage_MapRead;
    }
}

bitflags::bitflags! {
    struct DeviceBufferMapMode: WGPUMapMode {
        const READ = WGPUMapMode_WGPUMapMode_Read;
    }
}

impl DeviceBuffer {
    pub fn map_async(&self, mode: DeviceBufferMapMode, size: usize) -> oneshot::Receiver<()> {
        unsafe {
            unsafe extern "C" fn map_callback(
                res: WGPUBufferMapAsyncStatus,
                userdata: *mut c_void,
            ) {
                assert_eq!(
                    res,
                    WGPUBufferMapAsyncStatus_WGPUBufferMapAsyncStatus_Success
                );
                let mut tx = Box::from_raw(userdata as *mut Option<oneshot::Sender<()>>);
                (*tx).take().unwrap().send(()).unwrap();
            }

            let (tx, rx) = oneshot::channel::<()>();
            let tx = Box::new(Some(tx));

            wgpuBufferMapAsync(
                self.handle,
                mode.bits as _,
                0,
                size as _,
                Some(map_callback),
                Box::into_raw(tx) as _,
            );

            rx
        }
    }

    pub fn get_const_mapped_range(&self, size: usize) -> &[u8] {
        unsafe {
            let ptr = wgpuBufferGetConstMappedRange(self.handle, 0, size as _);
            std::slice::from_raw_parts(ptr as _, size)
        }
    }
}

impl Drop for DeviceBuffer {
    fn drop(&mut self) {
        unsafe {
            wgpuBufferRelease(self.handle);
        }
    }
}

struct BindGroupLayout {
    handle: WGPUBindGroupLayout,
}

impl BindGroupLayout {}

impl Drop for BindGroupLayout {
    fn drop(&mut self) {
        unsafe {
            wgpuBindGroupLayoutRelease(self.handle);
        }
    }
}

struct BindGroupEntry<'a> {
    binding: u32,
    buffer: &'a DeviceBuffer,
    size: usize,
}

impl<'a> From<&BindGroupEntry<'a>> for WGPUBindGroupEntry {
    fn from(entry: &BindGroupEntry<'a>) -> Self {
        WGPUBindGroupEntry {
            binding: entry.binding,
            buffer: entry.buffer.handle,
            offset: 0,
            size: entry.size as _,
            sampler: null_mut(),
            textureView: null_mut(),
            nextInChain: null(),
        }
    }
}

struct BindGroup {
    handle: WGPUBindGroup,
}

impl BindGroup {}

impl Drop for BindGroup {
    fn drop(&mut self) {
        unsafe {
            wgpuBindGroupRelease(self.handle);
        }
    }
}

struct CommandEncoder {
    handle: WGPUCommandEncoder,
}

impl CommandEncoder {
    pub fn begin_compute_pass(&self) -> ComputePassEncoder {
        unsafe {
            ComputePassEncoder {
                handle: wgpuCommandEncoderBeginComputePass(self.handle, &zeroed()),
            }
        }
    }

    pub fn copy_buffer_to_buffer(&self, src: &DeviceBuffer, dst: &DeviceBuffer, size: usize) {
        unsafe {
            wgpuCommandEncoderCopyBufferToBuffer(
                self.handle,
                src.handle,
                0,
                dst.handle,
                0,
                size as _,
            );
        }
    }

    pub fn finish(self) -> CommandBuffer {
        unsafe {
            CommandBuffer {
                handle: wgpuCommandEncoderFinish(self.handle, &zeroed()),
            }
        }
    }
}

impl Drop for CommandEncoder {
    fn drop(&mut self) {
        unsafe {
            wgpuCommandEncoderRelease(self.handle);
        }
    }
}

struct ComputePassEncoder {
    handle: WGPUComputePassEncoder,
}

impl ComputePassEncoder {
    pub fn set_pipeline(&self, pipeline: &ComputePipeline) {
        unsafe {
            wgpuComputePassEncoderSetPipeline(self.handle, pipeline.handle);
        }
    }

    pub fn set_bind_group(&self, index: u32, group: &BindGroup) {
        unsafe {
            wgpuComputePassEncoderSetBindGroup(self.handle, index, group.handle, 0, [].as_ptr());
        }
    }

    pub fn dispatch(&self, x: u32, y: u32, z: u32) {
        unsafe {
            wgpuComputePassEncoderDispatch(self.handle, x, y, z);
        }
    }
}

impl Drop for ComputePassEncoder {
    fn drop(&mut self) {
        unsafe {
            wgpuComputePassEncoderEndPass(self.handle);
            wgpuComputePassEncoderRelease(self.handle);
        }
    }
}

struct CommandBuffer {
    handle: WGPUCommandBuffer,
}

impl CommandBuffer {}

impl Drop for CommandBuffer {
    fn drop(&mut self) {
        unsafe {
            wgpuCommandBufferRelease(self.handle);
        }
    }
}

pub async fn run(shader: &str) -> Result<Buffer<1>> {
    let (device, queue) = Device::create();
    let shader_module = device.create_shader_module(shader);

    let compilation_info = shader_module.get_compilation_info().await;
    for msg in compilation_info.messages {
        println!("[{}:{}] {}", msg.line_number, msg.line_offset, msg.message);
    }

    if !compilation_info.success {
        panic!("shader compilation failed");
    }

    let pipeline = device.create_compute_pipeline(&shader_module, "main");

    let output = device.create_buffer(
        false,
        Buffer::<1>::SIZE,
        DeviceBufferUsage::STORAGE | DeviceBufferUsage::COPY_SRC,
    );

    let read = device.create_buffer(
        false,
        Buffer::<1>::SIZE,
        DeviceBufferUsage::COPY_DST | DeviceBufferUsage::MAP_READ,
    );

    let bind_group = device.create_bind_group(
        &pipeline.get_bind_group_layout(0),
        &[BindGroupEntry {
            binding: 0,
            buffer: &output,
            size: Buffer::<1>::SIZE,
        }],
    );

    let encoder = device.create_command_encoder();

    {
        let compute_pass = encoder.begin_compute_pass();
        compute_pass.set_pipeline(&pipeline);
        compute_pass.set_bind_group(0, &bind_group);
        compute_pass.dispatch(1, 1, 1);
    }

    encoder.copy_buffer_to_buffer(&output, &read, Buffer::<1>::SIZE);

    let commands = encoder.finish();

    queue.submit(&commands);

    let mut rx = read.map_async(DeviceBufferMapMode::READ, Buffer::<1>::SIZE);
    while rx.try_recv().unwrap().is_none() {
        device.tick();
        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    Ok(Buffer::from_bytes(
        read.get_const_mapped_range(Buffer::<1>::SIZE),
    ))
}
