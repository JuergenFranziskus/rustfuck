use crate::front_end::parser::{InstructionNode, NodeType};
use std::io::{Read, Write, ErrorKind};
use std::fmt::{Display, Formatter};
use std::time::Duration;


pub trait ByteSource {
    fn read(&mut self) -> Option<u8>;
}
pub struct StdInSource;
impl ByteSource for StdInSource {
    fn read(&mut self) -> Option<u8> {
        let mut buf = [0];

        let result = std::io::stdin().lock().read(&mut buf);

        match result {
            Ok(_size) => Some(buf[0]),
            Err(err) => {
                let kind = err.kind();
                match kind {
                    ErrorKind::UnexpectedEof => None,
                    _ => panic!("Failed to read input: {:?}", kind),
                }
            }
        }

    }
}

pub trait ByteWriter {
    fn write(&mut self, val: u8);
}
pub struct StdOutWriter;
impl ByteWriter for StdOutWriter {
    fn write(&mut self, val: u8) {
        let c = val as char;
        print!("{}", c);
        std::io::stdout().flush().unwrap();
    }
}




pub fn interpret<R, W>(node: &InstructionNode, out: &mut W, src: &mut R, sleep: Option<u32>) -> InterpretationResult
    where R: ByteSource,
          W: ByteWriter, {
    let mut context = Context {
        memory: Vec::with_capacity(30000),
        p: 0,
    };

    let sleep = match sleep {
        Some(time) => Some(Duration::from_millis(time as u64)),
        None => None,
    };

    context.interpret_node(node, out, src, sleep)
}


#[derive(Copy, Clone, Debug)]
pub enum InterpretationError {
    PointerUnderflow,
}
impl Display for InterpretationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PointerUnderflow => write!(f, "Cell pointer underflow on pointer decrement")?,
        }
        
        Ok(())
    }
}
pub type InterpretationResult = Result<(), InterpretationError>;



struct Context {
    memory: Vec<u8>,
    p: usize,
}
impl Context {
    fn expand_memory(&mut self) {
        while self.memory.len() <= self.p {
            self.memory.push(0);
        }
    }


    fn interpret_node<W, R>(&mut self, node: &InstructionNode, out: &mut W, src: &mut R, sleep: Option<Duration>) -> InterpretationResult
        where R: ByteSource,
              W: ByteWriter,
    {
        match &node.node_type {
            NodeType::Program(nodes) => {
                for child in nodes {
                    self.interpret_node(child, out, src, sleep)?;
                }
            }
            NodeType::Loop(nodes) => {
                loop {
                    self.expand_memory();
                    if self.memory[self.p] == 0 {
                        break;
                    } else {
                        for child in nodes {
                            self.interpret_node(child, out, src, sleep)?;
                        }
                    }
                }
            }
            NodeType::Next(amount) => {
                self.p += amount;
            }
            NodeType::Previous(amount) => {
                if self.p == 0 && *amount != 0 {
                    return Err(InterpretationError::PointerUnderflow);
                }

                self.p -= amount;
            }
            NodeType::Increment(amount) => {
                self.expand_memory();

                let cell = &mut self.memory[self.p];
                *cell = cell.wrapping_add((*amount % 256) as u8);
            }
            NodeType::Decrement(amount) => {
                self.expand_memory();

                let cell = &mut self.memory[self.p];
                *cell = cell.wrapping_sub((*amount % 256) as u8);
            }
            NodeType::Output => {
                self.expand_memory();

                let val = self.memory[self.p];
                out.write(val);
            }
            NodeType::Input => {
                self.expand_memory();

                if let Some(val) = src.read() {
                    self.memory[self.p] = val;
                }
            }

            NodeType::SetCell(val) => {
                self.expand_memory();

                self.memory[self.p] = (*val % 256) as u8;
            }
        }


        if let Some(time) = sleep {
            std::thread::sleep(time);
        }

        Ok(())
    }
}
