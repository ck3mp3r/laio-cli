use regex::Regex;

#[derive(Debug)]
pub(crate) struct Token {
    name: Option<String>,
    dimensions: Dimension,
    split: Option<SplitType>,
    // children: Vec<Token>,
}

#[derive(Debug)]
pub(crate) struct Dimension {
    width: i32,
    height: i32,
}

#[derive(Debug)]
pub(crate) enum SplitType {
    Vertical,
    Horizontal,
}

pub(crate) fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = vec![];
    for line in input.lines() {
        if let Some(token) = parse_window(line) {
            tokens.push(token)
        } else {
            log::error!("Unable to parse line: {}", line);
        }
    }
    tokens
}

fn parse_window(line: &str) -> Option<Token> {
    log::trace!("parse_window: {}", line);
    let re = Regex::new(r"^(\w+)\s+\w+,\s*(\d+)x(\d+)[,\d]+(.*)$").unwrap();
    let captures = re.captures(line)?;
    let name = captures.get(1).map_or("", |m| m.as_str());
    let width = captures.get(2)?.as_str().parse::<i32>().ok()?;
    let height = captures.get(3)?.as_str().parse::<i32>().ok()?;

    let mut contents = captures.get(4).map_or("", |m| m.as_str());
    let split = split_type(contents);
    if matches!(
        split,
        Some(SplitType::Vertical) | Some(SplitType::Horizontal)
    ) {
        log::trace!("split: {:?}", split);
        contents = contents[1..contents.len() - 1].trim();
    }
    log::trace!(
        "name: {}, width: {}, height: {}, split: {:?}, contents: {:?}",
        name,
        width,
        height,
        split,
        parse_pane(contents),
    );
    Some(Token {
        name: Some(name.to_string()),
        split,
        dimensions: Dimension { width, height },
        // children: parse_pane(contents),
    })
}

fn parse_pane(line: &str) -> Option<Token> {
    log::trace!("parse_pane: {}", line);
    let re = Regex::new(r"^(\d+)x(\d+)[,\d]+(.*)$").unwrap();
    let captures = re.captures(line)?;
    log::trace!("captures: {:?}", captures);
    let width = captures.get(1)?.as_str().parse::<i32>().ok()?;
    let height = captures.get(2)?.as_str().parse::<i32>().ok()?;

    let mut contents = captures.get(3).map_or("", |m| m.as_str());
    let split = split_type(contents);

    if matches!(
        split,
        Some(SplitType::Vertical) | Some(SplitType::Horizontal)
    ) {
        log::trace!("split: {:?}", split);
        contents = contents[1..contents.len() - 1].trim();
    }
    log::trace!(
        "width: {}, height: {}, split: {:?}, contents: {:?}",
        width,
        height,
        split,
        parse_pane(contents),
    );
    Some(Token {
        name: None,
        split,
        dimensions: Dimension { width, height },
        // children: parse_pane(contents),
    })
}

fn split_type(line: &str) -> Option<SplitType> {
    match line.chars().next() {
        Some('{') => Some(SplitType::Vertical),
        Some('[') => Some(SplitType::Horizontal),
        _ => None,
    }
}
