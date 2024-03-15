use std::fmt::format;

use blazesym::symbolize::CodeInfo;
use blazesym::symbolize::Input;
use blazesym::symbolize::Kernel;
use blazesym::symbolize::Process;
use blazesym::symbolize::Source;
use blazesym::symbolize::Sym;
use blazesym::symbolize::Symbolized;
use blazesym::symbolize::Symbolizer;
use blazesym::Addr;
use blazesym::Pid;
use rust_cpu_profiler_common::StackEvent;

const ADDR_WIDTH: usize = 16;

fn print_frame(
    name: &str,
    addr_info: Option<(Addr, Addr, usize)>,
    code_info: &Option<CodeInfo>,
) {
    let code_info = code_info.as_ref().map(|code_info| {
        let path = code_info.to_path();
        let path = path.display();

        match (code_info.line, code_info.column) {
            (Some(line), Some(col)) => format!(" {path}:{line}:{col}"),
            (Some(line), None) => format!(" {path}:{line}"),
            (None, _) => format!(" {path}"),
        }
    });

    if let Some((input_addr, addr, offset)) = addr_info {
        println!(
            "{input_addr:#0width$x}: {name} @ {addr:#x}+{offset:#x}{code_info}",
            code_info = code_info.as_deref().unwrap_or(""),
            width = ADDR_WIDTH
        )
    } else {
        println!(
            "{:width$}  {name}{code_info} [inlined]",
            " ",
            code_info = code_info
                .map(|info| format!(" @{info}"))
                .as_deref()
                .unwrap_or(""),
            width = ADDR_WIDTH
        )
    }
}

pub fn str_from_u8_nul_utf8(utf8_src: &[u8]) -> core::result::Result<&str, std::str::Utf8Error> {
    let nul_range_end = utf8_src
        .iter()
        .position(|&c| c == b'\0')
        .unwrap_or(utf8_src.len()); // default to length if no `\0` present
    ::std::str::from_utf8(&utf8_src[0..nul_range_end])
}

fn get_stack(symbolizer: Symbolizer, src: Source<'_>, addrs: &[u64], stack_size: i64) -> Vec<String> {
    let mut result = Vec::new(); // Vector to store the printed stack frames
    
    match symbolizer.symbolize(&src, Input::AbsAddr(addrs)) {
        Ok(syms) => {
            for (input_addr, sym) in addrs.iter().copied().zip(syms).take((stack_size / 8) as usize) {
                let frame_string = match sym {
                    Symbolized::Sym(Sym {
                        name,
                        addr,
                        offset,
                        code_info,
                        ..
                    }) => {
                        print_frame(&name, Some((input_addr, addr, offset)), &code_info);
                        let frame_string = format!("{};", name);
                        // for frame in inlined.iter() {
                        //     frame_string.push_str(&print_frame(&frame.name, None, &frame.code_info));
                        // }
                        frame_string
                    }
                    Symbolized::Unknown(..) => {
                        format!("{:#0width$x}: <no-symbol>", input_addr, width = ADDR_WIDTH)
                    }
                };
                result.push(frame_string); // Add the frame string to the result vector
            }
        }
        Err(e) => {
            result.push(format!("Error during symbolization: {:?}", e));
        }
    }
    result.last_mut().unwrap().pop();
    result // Return the vector containing the printed stack frames or error messages
}

//Return a Vec<StackInfo> StackInfo countains kstack combined ustack
pub fn resolve_sym(stack_event: &StackEvent) -> Vec<String>{
    let mut stack: Vec<String> = Vec::new();
    println!("{}", str_from_u8_nul_utf8(&stack_event.comm).unwrap().to_owned());
    stack.push(format!("{};", str_from_u8_nul_utf8(&stack_event.comm).unwrap().to_owned()));
    if stack_event.ustack_size > 0 {
        let symbolizer = Symbolizer::new();
        let src = Source::Process(Process::new(Pid::from(stack_event.pid as u32)));
        println!("User stack:");
        let addrs = stack_event.ustack;
        stack.append(&mut get_stack(symbolizer, src, &addrs, stack_event.ustack_size));
    }
    else {
        println!("No User stack found");
    }
    if stack_event.kstack_size > 0 {
        let symbolizer = Symbolizer::new();
        let src = Source::Kernel(Kernel::default());
        println!("Kernel stack:");
        let addrs = stack_event.kstack;
        stack.append(&mut get_stack(symbolizer, src, &addrs, stack_event.kstack_size));
    }
    else {
        println!("No Kernel stack found");
    }
    return stack;
}
