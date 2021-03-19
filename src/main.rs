#![allow(dead_code, unused_imports)]

use crate::front_end::lexer::{lex};
use crate::front_end::parser::{parse, print_tree, InstructionNode};
use std::io::stdout;
use crate::optimizer::apply_default_optimizations;
use crate::interpreter::{interpret, StdOutWriter, StdInSource};
use crate::compiler::compile_to_ir;
use clap::Clap;
use std::path::{PathBuf, Path};
use std::process::Command;

mod front_end;
mod interpreter;
mod optimizer;
mod compiler;

fn main() {
    let opts: Opts = Opts::parse();
    let source = match std::fs::read_to_string(&opts.input_path) {
        Ok(src) => src,
        Err(err) => {
            eprintln!("Failed to read input file {}: {}", opts.input_path, err);
            return;
        }
    };
    let tokens = lex(&source);
    let mut node = match parse(&tokens) {
        Ok(node) => node,
        Err(err) => {
            eprintln!("Failed to parse brainfuck program: {}", err);
            return;
        }
    };


    if !opts.disable_opt {
        apply_default_optimizations(&mut node);
    }



    if opts.interpret {
        let result = interpret(&node, &mut StdOutWriter, &mut StdInSource, opts.slow_down);

        if let Err(err) = result {
            eprintln!("\nEncountered error during execution: {}", err);
        }
    }
    else {
        match compile(&node, &opts) {
            Ok(()) => (),
            Err(()) => eprintln!("Compilation failed. Terminating..."),
        }
    }
}


fn compile(program: &InstructionNode, opts: &Opts) -> Result<(), ()> {
    let in_path = PathBuf::from(&opts.input_path);
    let mut out_path;
    if let Some(path) = &opts.output_path {
        out_path = PathBuf::from(path);
    }
    else {
        out_path = PathBuf::from("./");
    }

    let out_stem = match out_path.file_stem() {
        Some(stem) => stem.to_os_string(),
        None => in_path.file_stem().unwrap().to_os_string(),
    };
    let int_path = PathBuf::from(&opts.int_dir);
    let mut bc_path = int_path.clone();
    bc_path.push(format!("int_{}.bc", out_stem.to_str().unwrap()));
    let mut obj_path = int_path.clone();
    obj_path.push(format!("int_{}.o", out_stem.to_str().unwrap()));
    let mut flush_path = int_path;
    flush_path.push(format!("provint_flush_stdout_helper.o"));


    out_path.push(out_stem.clone());

    match std::fs::create_dir_all(bc_path.parent().unwrap()) {
        Ok(()) => (),
        Err(err) => {
            eprintln!("Failed to create intermediate directory: {}", err);
            return Err(());
        }
    }
    match std::fs::create_dir_all(out_path.parent().unwrap()) {
        Ok(()) => (),
        Err(err) => {
            eprintln!("Failed to create out directory: {}", err);
            return Err(());
        }
    }



    let bc_module = compile_to_ir(program, out_stem.to_str().unwrap());

    match std::fs::write(&bc_path, bc_module.as_slice()) {
        Ok(()) => (),
        Err(err) => {
            eprintln!("Failed to write bytecode file {}: {}", bc_path.to_str().unwrap(), err);
            return Err(());
        }
    };


    invoke_llc(&bc_path, &obj_path, opts)?;
    write_flush_helper(&flush_path)?;
    invoke_ld(&obj_path, &flush_path, &out_path)?;

    Ok(())
}

fn invoke_llc(bc_path: &Path, obj_path: &Path, opts: &Opts) -> Result<(), ()> {
    if opts.opt_level > 3 {
        eprintln!("Invalid optimization level: {}", opts.opt_level);
        return Err(())
    }

    match Command::new("llc")
        .arg("-o").arg(obj_path)
        .arg(format!("{}", bc_path.to_str().unwrap()))
        .arg("-filetype=obj")
        .arg(format!("-O{}", opts.opt_level))
        .output() {
        Ok(output) => {
            if !output.status.success() {
                eprintln!("Llc returned failure exit status:\n {} ", String::from_utf8_lossy(&output.stderr));
                return Err(())
            }
        },
        Err(err) => {
            eprintln!("Failed to invoke llc: {}", err);
            return Err(());
        }
    };

    Ok(())
}


const FLUSH_OBJ: &[u8] = include_bytes!("./helper/flush_stdout.o");
fn write_flush_helper(path: &Path) -> Result<(), ()> {
    match std::fs::write(path, FLUSH_OBJ) {
        Ok(()) => Ok(()),
        Err(err) => {
            eprintln!("Failed to write helper obj file {}: {}", path.to_str().unwrap(), err);
            Err(())
        }
    }
}

fn invoke_ld(obj_path: &Path, flush_path: &Path, out_path: &Path) -> Result<(), ()> {
    match Command::new("ld")
        .arg("-o").arg(out_path)
        .arg("-dynamic-linker").arg("/lib64/ld-linux-x86-64.so.2")
        .arg(obj_path)
        .arg(flush_path)
        .arg("-lc")
        .output() {
        Ok(output) => {
            if !output.status.success() {
                eprintln!("Ld returned failure exit status:\n {}", String::from_utf8_lossy(&output.stderr));
                return Err(());
            }
        }
        Err(err) => {
            eprintln!("Failed to invoke ld command: {}", err);
            return Err(());
        }
    };

    Ok(())
}





/// A semi-good brainfuck compiler/interpreter based on LLVM,
/// hastily thrown together in an afternoon.
/// No quality guaranteed!
#[derive(Clap, Debug)]
#[clap(version = "1.2", author = "Meryll")]
struct Opts {
    /// The path of the brainfuck file to compile/interpret.
    input_path: String,

    /// Interpret program instead of compiling.
    #[clap(short, long)]
    interpret: bool,

    /// The path of the executable file to write results to when compiling.
    #[clap(short)]
    output_path: Option<String>,

    /// Disables the internal optimizations of the brainfuck program.
    /// Does not affect llvm optimization level.
    #[clap(short, long)]
    disable_opt: bool,

    /// LLVM optimization level to use when compiling.
    /// Can be any of 0, 1, 2, 3.
    #[clap(short('O'), long, default_value = "2")]
    opt_level: u32,

    /// Directory to store intermediate files in.
    #[clap(short('I'), long("int"), default_value = "./int/")]
    int_dir: String,

    /// The amount of time to sleep after each instruction when interpreting, in milliseconds
    #[clap(short('s'), long("slowdown"))]
    slow_down: Option<u32>,
}
