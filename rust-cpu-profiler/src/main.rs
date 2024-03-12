use aya::programs::{perf_event, PerfEvent};
use aya::util::online_cpus;
use aya::{include_bytes_aligned, Bpf};
use log::debug;
use rust_cpu_profiler_common::StackEvent;
use aya::maps::RingBuf;
use std::convert::TryFrom;
use std::mem::size_of;
use std::ops::Deref;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

    // Bump the memlock rlimit. This is needed for older kernels that don't use the
    // new memcg based accounting, see https://lwn.net/Articles/837122/
    let rlim = libc::rlimit {
        rlim_cur: libc::RLIM_INFINITY,
        rlim_max: libc::RLIM_INFINITY,
    };
    let ret = unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlim) };
    if ret != 0 {
        debug!("remove limit on locked memory failed, ret is: {}", ret);
    }

    // This will include your eBPF object file as raw bytes at compile-time and load it at
    // runtime. This approach is recommended for most real-world use cases. If you would
    // like to specify the eBPF program at runtime rather than at compile-time, you can
    // reach for `Bpf::load_file` instead.
    #[cfg(debug_assertions)]
    let mut bpf = Bpf::load(include_bytes_aligned!(
        "../../target/bpfel-unknown-none/debug/rust-cpu-profiler"
    ))?;
    #[cfg(not(debug_assertions))]
    let mut bpf = Bpf::load(include_bytes_aligned!(
        "../../target/bpfel-unknown-none/release/rust-cpu-profiler"
    ))?;
    // if let Err(e) = BpfLogger::init(&mut bpf) {
    //     // This can happen if you remove all log statements from your eBPF program.
    //     warn!("failed to initialize eBPF logger: {}", e);
    // }
    // This will raise scheduled events on each CPU at 1 HZ, triggered by the kernel based
    // on clock ticks.
    let program: &mut PerfEvent = bpf.program_mut("rust_cpu_profiler").unwrap().try_into()?;
    program.load()?;
    for cpu in online_cpus()? {
        program.attach(
            perf_event::PerfTypeId::Software,
            perf_event::perf_sw_ids::PERF_COUNT_SW_CPU_CLOCK as u64,
            perf_event::PerfEventScope::AllProcessesOneCpu { cpu },
            perf_event::SamplePolicy::Frequency(1),
            true
        )?;
    }

    let mut ring_buf = RingBuf::try_from(bpf.map_mut("CPU_PROFILER").unwrap()).unwrap();

    loop {
        if let Some(item) = ring_buf.next() {
            let item_data: &[u8] = item.deref();
    
            if item_data.len() == size_of::<StackEvent>() {
                let stack_event: &StackEvent = unsafe { std::mem::transmute(item_data.as_ptr()) };
                
                symbol_resolver::resolve_sym(stack_event);
                // let comm = match std::str::from_utf8(&stack_event.comm) {
                //     Ok(s) => s.to_string(),
                //     Err(_) => {
                //         String::new()
                //     }
                // };
                // println!("comm: {}, cpu_id: {}", comm, my_struct.cpu_id);
                // if my_struct.kstack_size > 0{
                //     for sym in my_struct.kstack{
                //         println!("sym: {}", sym);
                //     }
                // }
            } else {
                println!("Error: Data length doesn't match struct size");
            }
        }

    }
}
