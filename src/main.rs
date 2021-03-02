#![allow(dead_code, unused_imports)]

use crate::front_end::lexer::{lex};
use crate::front_end::parser::{parse, print_tree};
use std::io::stdout;
use crate::optimizer::apply_default_optimizations;
use crate::interpreter::{interpret, StdOutWriter, StdInSource};
use crate::compiler::compile_to_ir;

mod front_end;
mod interpreter;
mod optimizer;
mod compiler;

fn main() {
    let src = std::fs::read_to_string("./programs/e.bf").unwrap();
    let tokens = lex(&src);
    let mut node = parse(&tokens).unwrap();

    //println!("Nodes before optimization:");
    //print_tree(&node, &mut stdout(), &String::new(), true).unwrap();
    //println!();

    apply_default_optimizations(&mut node);
    //println!("Nodes after optimization:");
    //print_tree(&node, &mut stdout(), &String::new(), true).unwrap();
    //println!();


    let compiled = compile_to_ir(&node, "hello_world");
    print!("{}", compiled);
}
