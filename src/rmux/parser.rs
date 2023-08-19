#[derive(Debug)]
pub(crate) enum SplitType {
    Horizontal,
    Vertical,
}

#[derive(Debug)]
pub(crate) struct Token {
    pub(crate) name: Option<String>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) split_type: Option<SplitType>,
    pub(crate) children: Vec<Token>,
}

fn extract_dimensions(segment: &str) -> (u32, u32) {
    let cleaned_segment: String = segment
        .chars()
        .filter(|c| c.is_numeric() || *c == 'x')
        .collect();
    let parts: Vec<&str> = cleaned_segment.split('x').collect();
    (parts[0].parse().unwrap(), parts[1].parse().unwrap())
}

pub(crate) fn tokenize(input: &str) -> Vec<Token> {
    let mut results = Vec::new();

    for line in input.lines() {
        let mut iter = line.splitn(2, ' ');
        let name = iter.next().unwrap().to_string();
        let remainder = iter.next().unwrap();

        results.push(tokenize_window(name, remainder));
    }

    results
}

fn tokenize_window(name: String, layout: &str) -> Token {
    let mut parts = layout.splitn(3, ',');

    let _ = parts.next().unwrap().to_string();
    let dimensions = extract_dimensions(parts.next().unwrap());

    let mut token = Token {
        name: Some(name),
        width: dimensions.0,
        height: dimensions.1,
        split_type: None,
        children: Vec::new(),
    };

    if let Some(remaining_layout) = parts.next() {
        token.children.extend(tokenize_pane(remaining_layout));
    }

    token
}

fn tokenize_pane(layout: &str) -> Vec<Token> {
    let mut results = Vec::new();
    let mut buffer = String::new();
    let mut depth = 0;

    for ch in layout.chars() {
        buffer.push(ch);

        match ch {
            '{' | '[' => {
                if depth == 0 {
                    buffer.pop(); // Remove the last character, it will be processed in the next depth
                    if !buffer.is_empty() {
                        results.extend(parse_buffer(&buffer));
                        buffer.clear();
                    }
                }
                depth += 1;
            }
            '}' | ']' => {
                depth -= 1;
                if depth == 0 {
                    let split = if ch == '}' {
                        SplitType::Horizontal
                    } else {
                        SplitType::Vertical
                    };
                    let mut token = Token {
                        name: None,
                        width: 0,  // Placeholder
                        height: 0, // Placeholder
                        split_type: Some(split),
                        children: Vec::new(),
                    };
                    buffer.pop(); // Remove the closing character
                    token.children.extend(tokenize_pane(&buffer));
                    results.push(token);
                    buffer.clear();
                }
            }
            _ => {}
        }
    }

    if !buffer.is_empty() {
        results.extend(parse_buffer(&buffer));
    }

    results
}

fn parse_buffer(buffer: &str) -> Vec<Token> {
    let mut results = Vec::new();
    for section in buffer.split(',') {
        let parts: Vec<&str> = section.splitn(2, 'x').collect();
        if parts.len() == 2 {
            let width = parts[0].parse().unwrap();
            let height = parts[1].parse().unwrap();
            results.push(Token {
                name: None,
                width,
                height,
                split_type: None,
                children: Vec::new(),
            });
        }
    }
    results
}
