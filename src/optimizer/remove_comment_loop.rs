use crate::front_end::parser::{InstructionNode, NodeType};




/// Removes any initial loops from a program node;
/// Since at the beginning of a program, a loop will always be skipped,
/// there is no reason to keep it.
/// Since it is still often likely to occur, being used to escape initial comments
/// in brainfuck programs, it should be somewhat reasonable to remove it.
/// This absolutely needs to be the first pass applied, or it might destroy actually relevant loops.
pub fn remove_comment_loop(program: &mut InstructionNode) {
    if let NodeType::Program(nodes) = &mut program.node_type {
        while nodes.len() != 0 &&
            matches!(nodes[0].node_type, NodeType::Loop(_)) {
            nodes.remove(0);
        }
    }
}
