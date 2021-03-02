use crate::front_end::parser::InstructionNode;
use remove_comment_loop::remove_comment_loop;
use collapse_increments::collapse_increments;
use collapse_decrements::collapse_decrements;
use collapse_next::collapse_next;
use collapse_previous::collapse_previous;
use collapse_set_zero::collapse_set_zero;

pub mod remove_comment_loop;
pub mod collapse_increments;
pub mod collapse_decrements;
pub mod collapse_next;
pub mod collapse_previous;
pub mod collapse_set_zero;

pub type OptimizerPass = fn(&mut InstructionNode);



pub fn apply_default_optimizations(program: &mut InstructionNode) {
    Optimizer::new()
        .with_pass(remove_comment_loop)
        .with_pass(collapse_increments)
        .with_pass(collapse_decrements)
        .with_pass(collapse_next)
        .with_pass(collapse_previous)
        .with_pass(collapse_set_zero)
        .apply(program);
}



pub struct Optimizer {
    passes: Vec<OptimizerPass>,
}
impl Optimizer {
    pub fn new() -> Optimizer {
        Optimizer {
            passes: Vec::new(),
        }
    }
    pub fn with_pass(mut self, pass: OptimizerPass) -> Optimizer {
        self.passes.push(pass);
        self
    }
    pub fn apply(self, program: &mut InstructionNode) {
        for pass in self.passes {
            pass(program);
        }
    }
}
