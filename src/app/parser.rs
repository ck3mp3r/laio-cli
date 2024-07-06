use regex::Regex;

#[derive(Debug, PartialEq)]
pub struct Dimensions {
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, PartialEq)]
pub struct Token {
    pub name: Option<String>,
    pub dimensions: Dimensions,
    pub split_type: Option<SplitType>,
    pub children: Vec<Token>,
}

#[derive(Debug, PartialEq)]
pub enum SplitType {
    Horizontal,
    Vertical,
}

impl SplitType {
    fn from_char(c: &char) -> Option<Self> {
        match c {
            '[' => Some(Self::Horizontal),
            '{' => Some(Self::Vertical),
            _ => None,
        }
    }

    fn closing_char(&self) -> char {
        match self {
            Self::Horizontal => ']',
            Self::Vertical => '}',
        }
    }
}

pub fn parse(input: &str) -> Vec<Token> {
    input
        .lines()
        .filter_map(|line| parse_window(line.trim()).map(|(token, _)| token))
        .collect()
}

fn parse_window(input: &str) -> Option<(Token, &str)> {
    let mut rest = input.trim_start();
    log::trace!("parse_window: {:?}", rest);

    let name_re = Regex::new(r"(?P<name>\w+)\s").unwrap();
    let dim_re = Regex::new(r"(?P<width>\d+)x(?P<height>\d+)[,\d]+").unwrap();

    let name = if let Some(captures) = name_re.captures(rest) {
        rest = &rest[captures.get(0).unwrap().end()..];
        log::trace!("rest-name {:?}", rest);
        Some(captures["name"].to_string())
    } else {
        None
    };
    log::trace!("name: {:?}", name);

    let dimensions = if let Some(captures) = dim_re.captures(rest) {
        rest = &rest[captures.get(0).unwrap().end()..];
        log::trace!("rest-dimensions {:?}", rest);
        Some(Dimensions {
            width: captures["width"].parse().unwrap(),
            height: captures["height"].parse().unwrap(),
        })
    } else {
        None
    }?;

    log::trace!("dimensions: {:?}", dimensions);

    let (children, split_type, rest) = if rest.is_empty() {
        (Vec::new(), None, rest)
    } else {
        parse_children(rest)
    };

    Some((
        Token {
            name,
            dimensions,
            split_type,
            children,
        },
        rest,
    ))
}

fn parse_children(input: &str) -> (Vec<Token>, Option<SplitType>, &str) {
    let mut rest = input.trim_start();
    log::trace!("parse_children: {:?}", rest);
    let mut children = Vec::new();

    let split_type = if let Some(c) = rest.chars().next() {
        if let Some(split_type) = SplitType::from_char(&c) {
            rest = &rest[1..];
            Some(split_type)
        } else {
            None
        }
    } else {
        None
    };
    log::trace!("split_type: {:?}", split_type);
    while !rest.is_empty()
        && split_type.is_some()
        && !rest.starts_with(split_type.as_ref().unwrap().closing_char())
    {
        log::trace!(
            "split_type: {:?}, {:?}",
            split_type,
            Some(split_type.as_ref().unwrap().closing_char())
        );
        if let Some((child, next_rest)) = parse_single(rest) {
            children.push(child);
            rest = next_rest;
        }
    }
    log::trace!("parse_children rest: {}", rest);
    if let Some(c) = rest.chars().next() {
        match &split_type {
            Some(split_type) if c == split_type.closing_char() => {
                rest = &rest[1..];
            }
            _ => {}
        }
    }
    (children, split_type, rest)
}

fn parse_single(input: &str) -> Option<(Token, &str)> {
    let mut rest = input.trim_start();
    log::trace!("parse_single: {:?}", rest);
    // Regular expressions
    let dim_re = Regex::new(r"(?P<width>\d+)x(?P<height>\d+)(,\d+){2,3}").unwrap();
    let dimensions = if let Some(captures) = dim_re.captures(rest) {
        rest = &rest[captures.get(0).unwrap().end()..];
        log::trace!("parse-single-rest-dimensions {:?}", rest);
        Some(Dimensions {
            width: captures["width"].parse().unwrap(),
            height: captures["height"].parse().unwrap(),
        })
    } else {
        None
    }?;
    log::trace!("dimensions {:?}", dimensions);

    let (children, split_type, rest) = if rest.is_empty() {
        (Vec::new(), None, rest)
    } else {
        parse_children(rest)
    };
    let token = Token {
        split_type,
        name: None,
        dimensions,
        children,
    };

    Some((token, rest))
}
