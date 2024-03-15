use anyhow::Ok;
use aya::programs::{perf_event, PerfEvent};
use aya::util::online_cpus;
use aya::{include_bytes_aligned, Bpf};
use inferno::flamegraph::{self, Options};
use log::debug;
use rust_cpu_profiler_common::StackEvent;
use aya::maps::{stack, RingBuf};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::{env, vec};
use std::mem::size_of;
use std::ops::Deref;
use std::time::Instant;
use tokio::io::unix::AsyncFd;
use tokio::task;
use std::path::{Path, PathBuf};
use std::vec::Vec;

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
            perf_event::SamplePolicy::Frequency(1000),
            true
        )?;
    }

    let args: Vec<String> = env::args().collect();
    
    let ring_buf = RingBuf::try_from(bpf.map_mut("CPU_PROFILER").unwrap()).unwrap();
    let mut poll_fd = AsyncFd::new(ring_buf)?;

    let timer = args[1].parse().unwrap();

    let started = Instant::now();

    let mut stack_count = HashMap::<Vec<String>, u64>::new();
    'outer: loop {
        let mut guard = poll_fd.readable_mut().await?;
        let ring_buf = guard.get_inner_mut();
        while let Some(item) = ring_buf.next() {
            if started.elapsed().as_secs() > timer{
                break 'outer ;
            }
            let item_data: &[u8] = item.deref();
            
            if item_data.len() == size_of::<StackEvent>() {
                let stack_event: &StackEvent = unsafe { std::mem::transmute(item_data.as_ptr()) };
                let vec_str: Vec<String> = symbol_resolver::resolve_sym(stack_event);

                *stack_count.entry(vec_str).or_insert(0) += 1;

            } else {
                println!("Error: Data length doesn't match struct size");
            }
        }
        guard.clear_ready();
    }
    for (stacks, counter)  in stack_count.iter() {
        println!("stack: {}, count {}", stacks.join(""), counter);
    }
    println!("stack_count size {}", stack_count.len());
    let mut svg_path: PathBuf = PathBuf::new();
    svg_path.push("/root/shared_folder/rust-cpu-profiler/flamegraph.svg");
    println!("svp path: {}", svg_path.to_string_lossy());
    let mut svg_file = std::io::BufWriter::with_capacity(1024 * 1024, std::fs::File::create(svg_path)?);
    let mut svg_opts = Options::default();
    svg_opts.title = String::from("My_flamegraph");

   //Create FlameGraph here
   let mut last_vec: Vec<String> = Vec::new();
   for (stacks, counter)  in stack_count.iter() {
    let concatenated_string = format!("{} {}", stacks.join(""), counter);
    last_vec.push(concatenated_string);
}
   flamegraph::from_lines(&mut svg_opts, last_vec.iter().map(|v| v.as_str()), &mut svg_file)?;

    Ok(())
}
