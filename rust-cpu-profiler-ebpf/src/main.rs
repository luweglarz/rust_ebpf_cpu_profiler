#![no_std]
#![no_main]

use core::mem::size_of_val;

use aya_bpf::{
    bindings::BPF_F_USER_STACK, cty::c_void, helpers::{bpf_get_current_pid_tgid, bpf_get_smp_processor_id, bpf_get_stack}, macros::{map, perf_event}, maps::RingBuf, programs::PerfEventContext, BpfContext
};
use rust_cpu_profiler_common::StackEvent;

#[map]
static CPU_PROFILER: RingBuf = RingBuf::with_byte_size(256 * 1024, 0); // 256 KB

#[perf_event]
pub fn rust_cpu_profiler(ctx: PerfEventContext) -> u32 {
    match try_rust_cpu_profiler(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_rust_cpu_profiler(ctx: PerfEventContext) -> Result<u32, u32> {
    if let Some(mut buf) = CPU_PROFILER.reserve::<StackEvent>(0) {
        unsafe {
                let buf_ptr: *mut StackEvent = buf.as_mut_ptr();
                (*buf_ptr).cpu_id = bpf_get_smp_processor_id();
                (*buf_ptr).pid = bpf_get_current_pid_tgid() >> 32;
                match ctx.command() {
                    Ok(comm) => {
                        (*buf_ptr).comm = comm;
                    }
                    Err(_error)  => {
                        (*buf_ptr).comm[0] = 0;
                    }
                }
                let kstack_ptr = (*buf_ptr).kstack.as_mut_ptr();
                let ustack_ptr = (*buf_ptr).ustack.as_mut_ptr();
                (*buf_ptr).kstack_size = bpf_get_stack(ctx.as_ptr(), kstack_ptr as *mut c_void, size_of_val(&(*buf_ptr).kstack) as u32, 0);
                (*buf_ptr).ustack_size = bpf_get_stack(ctx.as_ptr(), ustack_ptr as *mut c_void, size_of_val(&(*buf_ptr).ustack) as u32, BPF_F_USER_STACK.into());
        };
        buf.submit(2);
    }
    Ok(0)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
