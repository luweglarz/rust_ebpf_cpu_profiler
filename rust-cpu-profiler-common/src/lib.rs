#![no_std]

pub const TASK_COMM_LEN: usize = 16;

pub const MAX_STACK_DEPTH: usize = 127;

type StackTrace = [u64; MAX_STACK_DEPTH];

pub struct StackEvent {
    pub pid: u64,
    pub cpu_id: u32,
    pub comm: [u8; TASK_COMM_LEN],
    pub kstack_size: i64,
    pub ustack_size: i64,
    pub kstack: StackTrace,
    pub ustack: StackTrace
}