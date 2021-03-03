use crate::front_end::lexer::{Token, TokenType};
use std::fmt::{Display, Formatter};
use std::io::Write;

#[derive(Clone, Debug)]
pub enum NodeType {
    Program(Vec<InstructionNode>),
    Next(usize),
    Previous(usize),
    Increment(usize),
    Decrement(usize),
    Output,
    Input,
    Loop(Vec<InstructionNode>),

    // All following instructions are special-purpose for optimizing the above.
    SetCell(usize)
}

#[derive(Clone, Debug)]
pub struct InstructionNode {
    pub node_type: NodeType,
    pub line: u32,
    pub char: u32,
}



pub fn parse(tokens: &[Token]) -> Result<InstructionNode, ParsingError> {
    ParsingContext::new(tokens).parse_all()
}

struct ParsingContext<'a> {
    tokens: &'a [Token],
    index: usize,
}
impl<'a> ParsingContext<'a> {
    pub fn new(tokens: &'a [Token]) -> ParsingContext<'a> {
        ParsingContext {
            tokens,
            index: 0,
        }
    }
    pub fn parse_all(mut self) -> Result<InstructionNode, ParsingError> {
        let mut nodes = Vec::with_capacity(self.tokens.len());

        while !self.is_end() {
            nodes.push(self.parse_token()?);
        }


        Ok(InstructionNode {
            node_type: NodeType::Program(nodes),
            line: 0,
            char: 0
        })
    }


    fn current(&self) -> Token {
        self.tokens[self.index]
    }
    fn is_end(&self) -> bool {
        self.index >= self.tokens.len()
    }

    fn construct_node(&self, n_type: NodeType) -> InstructionNode {
        InstructionNode {
            node_type: n_type,
            line: self.current().line,
            char: self.current().char,
        }
    }


    fn parse_token(&mut self) -> Result<InstructionNode, ParsingError> {
        let c = self.current();

        let ret;

        match c.token_type {
            TokenType::Next => ret = self.construct_node(NodeType::Next(1)),
            TokenType::Previous => ret = self.construct_node(NodeType::Previous(1)),
            TokenType::Increment => ret = self.construct_node(NodeType::Increment(1)),
            TokenType::Decrement => ret = self.construct_node(NodeType::Decrement(1)),
            TokenType::Output => ret = self.construct_node(NodeType::Output),
            TokenType::Input => ret = self.construct_node(NodeType::Input),
            TokenType::BeginLoop => ret = self.parse_loop()?,
            TokenType::EndLoop => return Err(ParsingError::UnmatchedEndLoop {
                line: c.line,
                char: c.char,
            })
        }

        self.index += 1;

        Ok(ret)
    }
    fn parse_loop(&mut self) -> Result<InstructionNode, ParsingError> {
        let begin_line = self.current().line;
        let begin_char = self.current().char;

        self.index += 1;

        let mut children = Vec::new();

        loop {
            if self.is_end() {
                return Err(ParsingError::UnmatchedBeginLoop { line: begin_line, char: begin_char})
            }
            else if matches!(self.current().token_type, TokenType::EndLoop) {
                break;
            }
            else {
                children.push(self.parse_token()?);
            }
        }


        Ok(InstructionNode {
            node_type: NodeType::Loop(children),
            line: begin_line,
            char: begin_char,
        })
    }
}


#[derive(Copy, Clone, Debug)]
pub enum ParsingError {
    UnmatchedBeginLoop { line: u32, char: u32 },
    UnmatchedEndLoop { line: u32, char: u32 },
}
impl Display for ParsingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnmatchedBeginLoop { line, char} => {
                write!(f, "Opening [ on line {}, char {} has no closing ]", line, char)
            }
            Self::UnmatchedEndLoop { line, char } => {
                write!(f, "Closing ] on line {}, char {} has no opening [", line, char)
            }
        }
    }
}



pub fn print_tree<W: Write>(
    node: &InstructionNode,
    out: &mut W,
    indent: &String,
    last: bool,
) -> std::io::Result<()> {
    write!(out, "{}", indent)?;

    if last {
        write!(out, "└──")?;
    }
    else {
        write!(out, "├──")?;
    }

    let new_indent = if last {
        format!("{}   ", indent)
    }
    else {
        format!("{}│  ", indent)
    };


    match &node.node_type {
        NodeType::Next(amount) => writeln!(out, "Next({})", amount)?,
        NodeType::Previous(amount) => writeln!(out, "Previous({})", amount)?,
        NodeType::Increment(amount) => writeln!(out, "Increment({})", amount)?,
        NodeType::Decrement(amount) => writeln!(out, "Decrement({})", amount)?,
        NodeType::Output => writeln!(out, "Output")?,
        NodeType::Input => writeln!(out, "Input")?,
        NodeType::Loop(nodes) => {
            writeln!(out, "Loop:")?;

            for (i, n) in nodes.iter().enumerate() {
                if i == nodes.len() - 1 {
                    print_tree(n, out, &new_indent, true)?;
                }
                else {
                    print_tree(n, out, &new_indent, false)?;
                }
            }
        },
        NodeType::Program(nodes) => {
            writeln!(out, "Program:")?;

            for (i, n) in nodes.iter().enumerate() {
                if i == nodes.len() - 1 {
                    print_tree(n, out, &new_indent, true)?;
                }
                else {
                    print_tree(n, out, &new_indent, false)?;
                }
            }
        }

        NodeType::SetCell(amount) => writeln!(out, "SetCell({})", amount)?,
    }

    Ok(())
}
