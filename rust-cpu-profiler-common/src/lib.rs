#![no_std]

// https://elixir.bootlin.com/linux/v4.18/source/include/linux/sched.h, line 207
pub const TASK_COMM_LEN: usize = 16;

// /proc/sys/kernel/perf_event_max_stack 
pub const MAX_STACK_DEPTH: usize = 127;

pub struct StackEvent {
    pub pid: u64,
    pub cpu_id: u32,
    pub comm: [u8; TASK_COMM_LEN],
    pub kstack_size: i64,
    pub ustack_size: i64,
    pub kstack: [u64; MAX_STACK_DEPTH],
    pub ustack: [u64; MAX_STACK_DEPTH]
}