use std::{collections::HashMap, path::Path};

use log::trace;
use regex::Regex;

use crate::common::{
    config::{
        util::{gcd_vec, round},
        FlexDirection, Pane, Session, Window,
    },
    path::home_dir,
};

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Dimensions {
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub id: Option<String>,
    pub name: Option<String>,
    pub dimensions: Dimensions,
    pub path: Option<String>,
    pub split_type: Option<SplitType>,
    pub children: Vec<Token>,
    pub commands: Vec<String>,
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

impl FlexDirection {
    pub(crate) fn from_split_type(split_type: &SplitType) -> Self {
        match split_type {
            SplitType::Horizontal => Self::Column,
            SplitType::Vertical => Self::Row,
        }
    }
}

impl Window {
    fn from_tokens(token: &Token) -> Self {
        let pane_flex_direction = token
            .split_type
            .as_ref()
            .map(FlexDirection::from_split_type);
        Self {
            name: token.name.clone().unwrap_or_else(|| "foo".to_string()),
            flex_direction: pane_flex_direction.clone().unwrap_or_default(),
            panes: Pane::from_tokens(&token.children, pane_flex_direction.unwrap_or_default()),
        }
    }
}

impl Session {
    pub(crate) fn from_tokens(name: &str, path: &str, tokens: &[Token]) -> Self {
        Self {
            name: name.to_string(),
            startup: vec![],
            shutdown: vec![],
            env: HashMap::new(),
            path: path.to_string(),
            windows: tokens
                .iter()
                .map(|token| {
                    log::trace!("{:?}", token);
                    Window::from_tokens(token)
                })
                .collect(),
        }
    }
}

impl Pane {
    fn from_tokens(children: &[Token], flex_direction: FlexDirection) -> Vec<Pane> {
        if children.is_empty() {
            return vec![];
        }

        let dimension_selector = match flex_direction {
            FlexDirection::Row => |c: &Token| c.dimensions.width as usize,
            FlexDirection::Column => |c: &Token| c.dimensions.height as usize,
        };

        let dimensions: Vec<usize> = children.iter().map(dimension_selector).map(round).collect();

        let gcd = gcd_vec(&dimensions);
        log::trace!("gcd of dimensions: {:?}", gcd);

        // Calculate initial flex values
        let flex_values: Vec<usize> = children
            .iter()
            .map(|token| dimension_selector(token) / gcd)
            .collect();
        log::trace!("flex values: {:?}", flex_values);

        // Normalize flex values using the GCD
        let flex_gcd = gcd_vec(&flex_values);
        log::trace!("gcd of flex_values: {:?}", flex_gcd);

        // Creating panes with normalized flex values
        let panes: Vec<Pane> = children
            .iter()
            .zip(flex_values.iter())
            .map(|(token, &flex_value)| {
                let normalized_flex_value = (flex_value / flex_gcd).max(1);

                let pane_flex_direction = token
                    .split_type
                    .as_ref()
                    .map(FlexDirection::from_split_type)
                    .unwrap_or(FlexDirection::default());

                Pane {
                    flex_direction: pane_flex_direction.clone(),
                    flex: normalized_flex_value,
                    style: None,
                    path: match token.path {
                        Some(ref p) => p.clone(),
                        None => ".".to_string(),
                    },
                    commands: token.commands.clone(),
                    env: HashMap::new(),
                    panes: Pane::from_tokens(&token.children, pane_flex_direction),
                    zoom: false,
                }
            })
            .inspect(|pane| log::trace!("pane: {:?}", pane))
            .collect();

        panes
    }
}

pub fn parse(
    tmux_layout: &str,
    pane_paths: &HashMap<String, String>,
    session_path_str: &str,
    cmd_dict: &HashMap<String, String>,
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
        .filter_map(|line| parse_window(line.trim(), &adjusted_pane_paths, cmd_dict))
        .collect()
}

fn parse_window(
    input: &str,
    pane_paths: &HashMap<String, Option<String>>,
    cmd_dict: &HashMap<String, String>,
) -> Option<Token> {
    let mut rest = input.trim_start();
    trace!("line: {:?}", rest);
    trace!("parse_window: {:?}", rest);
    trace!("pane_paths: {:?}", pane_paths);

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
        trace!("rest-dimensions {:?}", rest);
        Some(Dimensions {
            width: captures["width"].parse().unwrap(),
            height: captures["height"].parse().unwrap(),
        })
    } else {
        None
    }?;

    trace!("dimensions: {:?}", dimensions);

    let (mut children, split_type, _) = parse_children(rest, pane_paths, cmd_dict);

    if children.is_empty() {
        let id_re = Regex::new(r"[,]{1}(?P<id>\d+)").unwrap();
        let id = if let Some(captures) = id_re.captures(rest) {
            rest = &rest[captures.get(0).unwrap().end()..];
            trace!("id-rest: {:?}", rest);
            Some(captures["id"].parse::<String>().unwrap())
        } else {
            None
        };
        trace!("id: {:?}", id);

        if let Some(id) = id {
            let path = pane_paths.get(&id).and_then(|opt| opt.clone());
            let commands = cmd_dict
                .get(&id)
                .map(|cmd| cmd.to_string())
                .map_or(vec![], |cmd| vec![cmd]);

            if path.is_some() || !commands.is_empty() {
                children.push(Token {
                    id: Some(id),
                    name: None,
                    dimensions,
                    path,
                    split_type: None,
                    children: vec![],
                    commands,
                });
            }
        }
    }

    Some(Token {
        id: None,
        name,
        dimensions,
        path: None,
        split_type,
        children,
        commands: vec![],
    })
}

fn parse_children<'a>(
    input: &'a str,
    pane_paths: &HashMap<String, Option<String>>,
    cmd_dict: &HashMap<String, String>,
) -> (Vec<Token>, Option<SplitType>, &'a str) {
    let mut rest = input.trim_start();
    trace!("parse_children: {:?}", rest);
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
        if let Some((child, next_rest)) = parse_single(rest, pane_paths, cmd_dict) {
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
    cmd_dict: &HashMap<String, String>,
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

    let (children, split_type, rest) = parse_children(rest, pane_paths, cmd_dict);

    let (path, commands) = if children.is_empty() {
        let path = match pane_paths.get(&dimensions_pane_id.1) {
            Some(Some(path)) => {
                trace!("path: {:?}", path);
                Some(path.clone())
            }
            Some(None) | None => {
                trace!("path: None");
                None
            }
        };

        let cmds = match cmd_dict.get(&dimensions_pane_id.1) {
            Some(cmd) => {
                vec![cmd.to_string()]
            }
            None => vec![],
        };

        (path, cmds)
    } else {
        (None, vec![])
    };

    let token = Token {
        id: Some(dimensions_pane_id.1),
        split_type,
        name: None,
        path,
        dimensions: dimensions_pane_id.0,
        children,
        commands,
    };

    Some((token, rest))
}
