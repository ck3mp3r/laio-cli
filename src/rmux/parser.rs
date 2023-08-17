enum Token {
    Identifier(String),
    Number(u32),
    Symbol(char),
    OpenCurly,
    CloseCurly,
    OpenSquare,
    CloseSquare,
}

fn tokenize(input: &str) -> Vec<Token>{
    let mut tokens = Vec::new();

    tokens
}
