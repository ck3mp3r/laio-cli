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
            '{' => Some(Self::Horizontal),
            '[' => Some(Self::Vertical),
            _ => None,
        }
    }
}

impl Token {
    pub fn parse(input: &str) -> Vec<Self> {
        input
            .lines()
            .filter_map(|line| Token::parse_window(line.trim()).map(|(token, _)| token))
            .collect()
    }

    fn parse_window(input: &str) -> Option<(Self, &str)> {
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

        let (children, split_type) = if rest.is_empty() {
            (Vec::new(), None)
        } else {
            Self::parse_children(rest)
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

    fn parse_children(input: &str) -> (Vec<Token>, Option<SplitType>) {
        let mut rest = input.trim_start();
        log::trace!("parse_children: {:?}", rest);
        let mut children = Vec::new();
        while !rest.is_empty() && (rest.starts_with('{') || rest.starts_with('[')) {
            log::trace!("rest: {}", rest);
            if let Some((child, next_rest)) = Token::parse_single(rest) {
                children.push(child);
                rest = next_rest;
            } else {
                return (children, None);
            }
            rest = rest[1..].trim_start();
        }
        (children, None)
    }

    fn parse_single(input: &str) -> Option<(Self, &str)> {
        let mut rest = input.trim_start();
        log::trace!("parse_single: {:?}", rest);
        // Regular expressions
        let dim_re = Regex::new(r"(?P<width>\d+)x(?P<height>\d+)[,\d]2:3").unwrap();
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
        log::trace!("dimensions {:?}", dimensions);

        let (children, _) = if rest.is_empty() {
            (Vec::new(), None)
        } else {
            Self::parse_children(rest)
        };
        let token = Token {
            split_type: None,
            name: None,
            dimensions,
            children,
        };

        Some((token, rest))
    }
}
