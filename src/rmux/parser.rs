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
    pub children: Vec<Token>,
}

impl Token {
    pub fn parse(input: &str) -> Vec<Self> {
        input
            .lines()
            .filter_map(|line| Token::parse_single(line.trim()).map(|(token, _)| token))
            .collect()
    }

    fn parse_single(input: &str) -> Option<(Self, &str)> {
        // Regular expressions
        let name_re = Regex::new(r"(?P<name>\w+)\s").unwrap();
        let dim_re = Regex::new(r"(?P<width>\d+)x(?P<height>\d+)").unwrap();

        let mut rest = input.trim_start();

        let name = if let Some(captures) = name_re.captures(rest) {
            rest = &rest[captures.get(0).unwrap().end()..];
            Some(captures["name"].to_string())
        } else {
            None
        };

        let dimensions = if let Some(captures) = dim_re.captures(rest) {
            rest = &rest[captures.get(0).unwrap().end()..];
            Some(Dimensions {
                width: captures["width"].parse().unwrap(),
                height: captures["height"].parse().unwrap(),
            })
        } else {
            None
        }?;

        // Skip any non-pertinent characters (numbers and commas)
        while let Some(next_char) = rest.chars().next() {
            if next_char.is_numeric() || next_char == ',' {
                rest = &rest[1..];
            } else {
                break;
            }
        }
        log::debug!("rest: {}", rest);
        let mut children = Vec::new();
        while !rest.is_empty() && (rest.starts_with('{') || rest.starts_with('[')) {
            log::trace!("rest: {}", rest);
            if let Some((child, next_rest)) = Token::parse_single(rest) {
                children.push(child);
                rest = next_rest;
            } else {
                return None;
            }
            rest = rest[1..].trim_start();
        }

        let token = Token {
            name,
            dimensions,
            children,
        };

        Some((token, rest))
    }
}
