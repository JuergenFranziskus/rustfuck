use crate::front_end::parser::{InstructionNode, NodeType};



pub fn collapse_next(program: &mut InstructionNode) {
    match &mut program.node_type {
        NodeType::Program(nodes) => collapse_node_list(nodes),
        NodeType::Loop(nodes) => collapse_node_list(nodes),
        _ => (),
    }
}


fn collapse_node_list(nodes: &mut Vec<InstructionNode>) {
    let mut new_nodes = Vec::with_capacity(nodes.len());


    let mut current_incr = None;
    let mut current_line = 0;
    let mut current_char = 0;


    for mut node in nodes.split_off(0).into_iter() {
        if let NodeType::Next(amount) = node.node_type {
            match &mut current_incr {
                Some(incr) => *incr += amount,
                None => {
                    current_incr = Some(amount);
                    current_line = node.line;
                    current_char = node.char;
                }
            }
        }
        else {
            if let Some(incr) = current_incr.take() {
                new_nodes.push(InstructionNode {
                    node_type: NodeType::Next(incr),
                    line: current_line,
                    char: current_char,
                });
            }

            collapse_next(&mut node);

            new_nodes.push(node);
        }
    }
    if let Some(incr) = current_incr.take() {
        new_nodes.push(InstructionNode {
            node_type: NodeType::Next(incr),
            line: current_line,
            char: current_char,
        });
    }

    *nodes = new_nodes;
}

