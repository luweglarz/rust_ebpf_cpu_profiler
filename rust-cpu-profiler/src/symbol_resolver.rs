// use aya::maps::stack;
// use blazesym::symbolize::CodeInfo;
// use blazesym::symbolize::Input;
// use blazesym::symbolize::Kernel;
// use blazesym::symbolize::Process;
// use blazesym::symbolize::Source;
// use blazesym::symbolize::Sym;
// use blazesym::symbolize::Symbolized;
// use blazesym::symbolize::Symbolizer;
// use blazesym::Addr;
// use blazesym::Pid;
 use rust_cpu_profiler_common::StackEvent;

// const ADDR_WIDTH: usize = 16;

// fn print_frame(
//     name: &str,
//     addr_info: Option<(Addr, Addr, usize)>,
//     code_info: &Option<CodeInfo>,
// ) {
//     let code_info = code_info.as_ref().map(|code_info| {
//         let path = code_info.to_path();
//         let path = path.display();

//         match (code_info.line, code_info.column) {
//             (Some(line), Some(col)) => format!(" {path}:{line}:{col}"),
//             (Some(line), None) => format!(" {path}:{line}"),
//             (None, _) => format!(" {path}"),
//         }
//     });

//     if let Some((input_addr, addr, offset)) = addr_info {
//         println!(
//             "{input_addr:#0width$x}: {name} @ {addr:#x}+{offset:#x}{code_info}",
//             code_info = code_info.as_deref().unwrap_or(""),
//             width = ADDR_WIDTH
//         )
//     } else {
//         println!(
//             "{:width$}  {name}{code_info} [inlined]",
//             " ",
//             code_info = code_info
//                 .map(|info| format!(" @{info}"))
//                 .as_deref()
//                 .unwrap_or(""),
//             width = ADDR_WIDTH
//         )
//     }
// }

// fn print_stack(symbolizer: Symbolizer, src: Source<'_>, addrs: &[u64], stack_size: i64){
//     let syms = symbolizer.symbolize(&src, Input::AbsAddr(addrs)).unwrap();

//     for (input_addr, sym) in addrs.iter().copied().zip(syms).take((stack_size / 8) as usize) {
//         match sym {
//             Symbolized::Sym(Sym {
//                 name,
//                 addr,
//                 offset,
//                 code_info,
//                 inlined,
//                 ..
//             }) => {
//                 print_frame(&name, Some((input_addr, addr, offset)), &code_info);
//                 for frame in inlined.iter() {
//                     print_frame(&frame.name, None, &frame.code_info);
//                 }
//             }
//             Symbolized::Unknown(..) => {
//                 println!("{input_addr:#0width$x}: <no-symbol>", width = ADDR_WIDTH)
//             }
//         }
//     }  
// }

// pub fn resolve_sym(stack_event: &StackEvent){
//     let src = Source::Process(Process::new(Pid::from(stack_event.pid as u32)));
//     let symbolizer = Symbolizer::new();

//     if stack_event.pid == 0{
//         println!("{} Kernel stack:", String::from_utf8_lossy(&stack_event.comm));
//         let addrs = stack_event.kstack;
//         print_stack(symbolizer, src, &addrs, stack_event.kstack_size);
//     }
//     else {
//         println!("{} User stack:", String::from_utf8_lossy(&stack_event.comm));
//         let addrs = stack_event.ustack;
//         print_stack(symbolizer, src, &addrs, stack_event.ustack_size);
//     }

// }

use std::collections::BTreeMap;

use aya::util::kernel_symbols;

trait Resolver{
    fn resolve_stack(&self, stack_event: &StackEvent);
}

struct UserSymbolResolver {

}

impl UserSymbolResolver {
    pub fn new() -> Self {
        Self {
        }
    }
}

impl Resolver for UserSymbolResolver{
    fn resolve_stack(&self, stack_event: &StackEvent) {
        
    }
}

struct KernelSymbolResolver {
    kallsyms: BTreeMap<u64, String>
}

impl KernelSymbolResolver {
    pub fn new() -> Self {
        Self {
            kallsyms: kernel_symbols().unwrap(),
        }
    }
}

impl Resolver for KernelSymbolResolver{
    fn resolve_stack(&self, stack_event: &StackEvent) {
        for frame in stack_event.kstack.iter().copied().take((stack_event.kstack_size / 8) as usize) {
            if let Some(sym) = self.kallsyms.range(..=frame).next_back().map(|(_, s)| s) {
                println!(
                    "{:#x} {}",
                    frame,
                    sym
                );
            } else {
                println!(
                    "{:#x}",
                    frame
                );
            }
        }
    }
}

pub fn resolve_sym(stack_event: &StackEvent){
    println!("{}", String::from_utf8_lossy(&stack_event.comm));
    if stack_event.kstack_size > 0{
        let resolver: KernelSymbolResolver = KernelSymbolResolver::new();
        println!("Kernel stack:");
        resolver.resolve_stack(stack_event);
    }
    else{
        println!("No kernel stack ");
    }

    if stack_event.ustack_size > 0{
        let resolver: UserSymbolResolver = UserSymbolResolver::new();
        println!("User stack:");
        resolver.resolve_stack(stack_event);
    }
    else{
        println!("No user stack");
    }
}