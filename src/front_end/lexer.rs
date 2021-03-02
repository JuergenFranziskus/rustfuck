use std::io::Write;


#[derive(Copy, Clone, Debug)]
pub enum TokenType {
    Next,
    Previous,
    Increment,
    Decrement,
    Output,
    Input,
    BeginLoop,
    EndLoop,
}
#[derive(Copy, Clone, Debug)]
pub struct Token {
    pub token_type: TokenType,
    pub line: u32,
    pub char: u32,
}

pub fn lex(src: &str) -> Vec<Token> {
    let mut line = 1;
    let mut char = 1;


    let mut tokens = Vec::new();


    for c in src.chars() {
        match c {
            '\n' => {
                line += 1;
                char = 1;
                continue;
            }

            '>' => tokens.push(Token {
                token_type: TokenType::Next,
                line,
                char,
            }),
            '<' => tokens.push(Token {
                token_type: TokenType::Previous,
                line,
                char
            }),
            '+' => tokens.push(Token {
                token_type: TokenType::Increment,
                line,
                char
            }),
            '-' => tokens.push(Token {
                token_type: TokenType::Decrement,
                line,
                char,
            }),
            '.' => tokens.push(Token {
                token_type: TokenType::Output,
                line,
                char,
            }),
            ',' => tokens.push(Token {
                token_type: TokenType::Input,
                line,
                char
            }),
            '[' => tokens.push(Token {
                token_type: TokenType::BeginLoop,
                line,
                char
            }),
            ']' => tokens.push(Token {
                token_type: TokenType::EndLoop,
                line,
                char,
            }),

            _ => (),
        }

        char += 1;
    }


    tokens
}



pub fn print_tokens<W: Write>(tokens: &[Token], out: &mut W) -> std::io::Result<()> {
    let mut type_strings = Vec::with_capacity(tokens.len());
    let mut line_strings = Vec::with_capacity(tokens.len());
    let mut char_strings = Vec::with_capacity(tokens.len());

    let mut type_len = 0;
    let mut line_len = 0;
    let mut char_len = 0;

    for token in tokens {
        let type_string = format!("{:?}", token.token_type);
        let line_string = format!("{}", token.line);
        let char_string = format!("{}", token.char);

        type_len = type_len.max(type_string.len());
        line_len = line_len.max(line_string.len());
        char_len = char_len.max(char_string.len());

        type_strings.push(type_string);
        line_strings.push(line_string);
        char_strings.push(char_string);
    }


    for ((type_str, line_str), char_str) in type_strings.iter().zip(line_strings.iter()).zip(char_strings.iter()) {
        writeln!(
            out,
            "Token: {:<tw$} ({:0>lw$}|{:<cw$})",
            type_str, line_str, char_str, tw = type_len, lw = line_len, cw = char_len,
        )?;
    }

    Ok(())
}

