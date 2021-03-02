use crate::front_end::parser::{InstructionNode, NodeType};




pub fn collapse_set_zero(node: &mut InstructionNode) {
    match &mut node.node_type {
        NodeType::Program(children) => collapse_nodes(children),
        NodeType::Loop(children) => collapse_nodes(children),
        _ => (),
    }
}
fn collapse_nodes(nodes: &mut Vec<InstructionNode>) {
    for node in nodes {
        if let NodeType::Loop(children) = &node.node_type {
            if children.len() == 1 {
                if let NodeType::Decrement(1) = children[0].node_type {
                    *node = InstructionNode {
                        node_type: NodeType::SetCell(0),
                        line: node.line,
                        char: node.char,
                    };

                }
            }
            else {
                collapse_set_zero(node);
            }
        }
        else {
            collapse_set_zero(node);
        }
    }
}
