use std::{collections::HashMap, path::Path};

use log::{debug, trace};
use regex::Regex;

use crate::util::path::home_dir;

#[derive(Debug, PartialEq, Clone)]
pub struct Dimensions {
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub name: Option<String>,
    pub dimensions: Dimensions,
    pub path: Option<String>,
    pub split_type: Option<SplitType>,
    pub children: Vec<Token>,
}

#[derive(Debug, PartialEq, Clone)]
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

pub fn parse(
    tmux_layout: &str,
    pane_paths: &HashMap<String, String>,
    session_path_str: &str,
) -> Vec<Token> {
    let home_dir = home_dir().unwrap_or_else(|_| "/".to_string());
    let session_path_str = session_path_str.replace(&home_dir, "~");
    let session_path = Path::new(&session_path_str);
    log::trace!("session_path: {:?}", session_path);

    let mut adjusted_pane_paths: HashMap<String, Option<String>> = HashMap::new();
    for (pane_id, full_path_str) in pane_paths {
        let full_path_str = full_path_str.replace(&home_dir, "~");
        let full_path = Path::new(&full_path_str);
        let relative_path = full_path.strip_prefix(session_path).unwrap_or(full_path);
        let path_str = relative_path.to_string_lossy().into_owned();
        let path_opt = if path_str.is_empty() {
            None
        } else {
            Some(path_str)
        };
        log::trace!("path: {:?}", path_opt);
        adjusted_pane_paths.insert(pane_id.clone(), path_opt);
    }

    // Process the tmux_layout with the adjusted pane paths
    tmux_layout
        .lines()
        .filter_map(|line| parse_window(line.trim(), &adjusted_pane_paths))
        .collect()
}

fn parse_window(input: &str, pane_paths: &HashMap<String, Option<String>>) -> Option<Token> {
    let mut rest = input.trim_start();
    debug!("line: {:?}", rest);
    trace!("parse_window: {:?}", rest);
    debug!("pane_paths: {:?}", pane_paths);

    let name_re = Regex::new(r"(?P<name>\w+)\s").unwrap();
    let name = if let Some(captures) = name_re.captures(rest) {
        rest = &rest[captures.get(0).unwrap().end()..];
        trace!("rest-name {:?}", rest);
        Some(captures["name"].to_string())
    } else {
        None
    };
    trace!("name: {:?}", name);

    let dim_re = Regex::new(r"(?P<width>\d+)x(?P<height>\d+)(,\d){2}").unwrap();
    let dimensions = if let Some(captures) = dim_re.captures(rest) {
        rest = &rest[captures.get(0).unwrap().end()..];
        debug!("rest-dimensions {:?}", rest);
        Some(Dimensions {
            width: captures["width"].parse().unwrap(),
            height: captures["height"].parse().unwrap(),
        })
    } else {
        None
    }?;

    trace!("dimensions: {:?}", dimensions);

    let (mut children, split_type, _) = parse_children(rest, pane_paths);

    if children.is_empty() {
        let id_re = Regex::new(r"[,]{1}(?P<id>\d+)").unwrap();
        let id = if let Some(captures) = id_re.captures(rest) {
            rest = &rest[captures.get(0).unwrap().end()..];
            debug!("id-rest: {:?}", rest);
            Some(captures["id"].parse::<String>().unwrap())
        } else {
            None
        };
        debug!("id: {:?}", id);
        
        if id.is_some() {
            let pane_path = pane_paths.get(&id.unwrap());
            children.push(Token {
                name: None,
                dimensions: dimensions.clone(),
                path: pane_path.unwrap().clone(),
                split_type: None,
                children: [].to_vec(),
            });
        };
    }

    Some(Token {
        name,
        dimensions,
        path: None,
        split_type,
        children,
    })
}

fn parse_children<'a>(
    input: &'a str,
    pane_paths: &HashMap<String, Option<String>>,
) -> (Vec<Token>, Option<SplitType>, &'a str) {
    let mut rest = input.trim_start();
    debug!("parse_children: {:?}", rest);
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
    trace!("split_type: {:?}", split_type);
    while !rest.is_empty()
        && split_type.is_some()
        && !rest.starts_with(split_type.as_ref().unwrap().closing_char())
    {
        trace!(
            "split_type: {:?}, {:?}",
            split_type,
            Some(split_type.as_ref().unwrap().closing_char())
        );
        if let Some((child, next_rest)) = parse_single(rest, pane_paths) {
            children.push(child);
            rest = next_rest;
        }
    }
    trace!("parse_children rest: {}", rest);
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

fn parse_single<'a>(
    input: &'a str,
    pane_paths: &HashMap<String, Option<String>>,
) -> Option<(Token, &'a str)> {
    let mut rest = input.trim_start();
    trace!("parse_single: {:?}", rest);
    let dim_re = Regex::new(r"(?P<width>\d+)x(?P<height>\d+)(,\d+){1,2},(?P<id>\d+)").unwrap();
    let dimensions_pane_id = if let Some(captures) = dim_re.captures(rest) {
        rest = &rest[captures.get(0).unwrap().end()..];
        trace!("parse-single-rest-dimensions {:?}", rest);
        Some((
            Dimensions {
                width: captures["width"].parse().unwrap(),
                height: captures["height"].parse().unwrap(),
            },
            captures["id"].parse::<String>().unwrap(),
        ))
    } else {
        None
    }?;
    trace!("dimensions and pane id {:?}", dimensions_pane_id);

    let (children, split_type, rest) = parse_children(rest, pane_paths);

    let path = if children.is_empty() {
        match pane_paths.get(&dimensions_pane_id.1) {
            Some(Some(path)) => {
                trace!("path: {:?}", path);
                Some(path.clone()) // Clone the path to return an owned String
            }
            Some(None) | None => {
                trace!("path: None");
                None
            }
        }
    } else {
        None
    };

    let token = Token {
        split_type,
        name: None,
        path,
        dimensions: dimensions_pane_id.0,
        children,
    };

    Some((token, rest))
}
