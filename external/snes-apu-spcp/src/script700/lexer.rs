use std::sync::Arc;
use super::context::ScriptContext;
use super::tokenizer::TokenizerAdapter;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Token<'a> {
    Invalid(&'a str),
    Command(&'a str),
    Parameter(&'a str),
    Operation(&'a str),
    Data(&'a str),
    ImportPath(&'a str)
}

pub type Line<'a> = Vec<Token<'a>>;

fn tokenize_script_line(source_line: &str, context: Arc<ScriptContext>) -> (Line<'_>, bool) {
    let mut tokens = Line::new();
    let mut area_ended = false;

    for image in source_line.tokenize() {
        // Comments
        if image.starts_with(';') {
            break;
        }

        // Import paths
        if image.starts_with('"') && image.ends_with('"') {
            tokens.push(Token::ImportPath(&image[1..(image.len() - 1)]));
            continue;
        }

        // Special cases
        match image {
            // Operations
            "+" | "-" | "*" | "/" | "\\" | "%" | "$" | "&" | "|" | "^" | "<" | "_" | ">" | "!" => {
                tokens.push(Token::Operation(image));
                continue;
            },

            // End of area
            "e" => {
                tokens.push(Token::Command("e"));
                area_ended = true;
                break;
            },

            // Weed out commands that contain numbers
            "r0" | "r1" | "f0" | "f1" => {
                tokens.push(Token::Command(image));
                continue;
            },

            // Imports
            "#i" | "#it" => {
                tokens.push(Token::Command(image));
                continue;
            },

            _ => ()
        }

        // Labels
        if image.starts_with(':') {
            tokens.push(Token::Command(":"));
            tokens.push(Token::Parameter(&image[1..]));
            continue;
        }

        // Now, if the token contains a number or a ? (dynamic parameter), it is a parameter
        if image.chars().any(|c| c.is_numeric() || c == '?') || (image.contains('$') && image.chars().any(|c| c.is_ascii_hexdigit())) {
            tokens.push(Token::Parameter(image));
            continue;
        }

        if image.chars().all(|c| c.is_alphabetic()) {
            tokens.push(Token::Command(image));
            continue;
        }

        println!("[Script700] {}: parse error: invalid token '{}'", context, image);
        tokens.push(Token::Invalid(image));
    }

    (tokens, area_ended)
}

fn tokenize_script_area<'i, 'a: 'i, I: Iterator<Item = (usize, &'a str)>>(source_lines: &'i mut I, file: &'a str, is_import: bool) -> Vec<(Token<'a>, Arc<ScriptContext>)> {
    let mut result: Vec<(Token<'a>, Arc<ScriptContext>)> = Vec::new();

    loop {
        let (line_number, source_line) = match source_lines.next() {
            Some(line) => line,
            None => break
        };
        let context = Arc::new(ScriptContext::new(file, line_number, is_import));
        let (line, area_finished) = tokenize_script_line(source_line, context.clone());
        for token in line {
            result.push((token, context.clone()));
        }
        if area_finished {
            break
        }
    }

    result
}

fn tokenize_extended_line(source_line: &str, context: Arc<ScriptContext>) -> (Line<'_>, bool) {
    let mut tokens = Line::new();
    let mut area_ended = false;

    for image in source_line.split_whitespace() {
        match image {
            "::" => {
                tokens.push(Token::Command("::"));
                continue;
            },
            "e" => {
                tokens.push(Token::Command(image));
                area_ended = true;
                break;
            },

            _ => ()
        }

        println!("[Script700] {}: parse error: invalid token '{}'", context, image);
        tokens.push(Token::Invalid(image));
    }

    (tokens, area_ended)
}

fn tokenize_extended_area<'i, 'a: 'i, I: Iterator<Item = (usize, &'a str)>>(source_lines: &'i mut I, file: &'a str, is_import: bool) -> Vec<(Token<'a>, Arc<ScriptContext>)> {
    let mut result: Vec<(Token<'a>, Arc<ScriptContext>)> = Vec::new();

    loop {
        let (line_number, source_line) = match source_lines.next() {
            Some(line) => line,
            None => break
        };
        let context = Arc::new(ScriptContext::new(file, line_number, is_import));
        let (line, area_finished) = tokenize_extended_line(source_line, context.clone());
        for token in line {
            result.push((token, context.clone()));
        }
        if area_finished {
            break
        }
    }

    result
}

fn tokenize_data_line(source_line: &str, context: Arc<ScriptContext>) -> Line<'_> {
    let mut tokens = Line::new();

    for image in source_line.split_whitespace() {
        // Labels
        if image.starts_with(':') {
            tokens.push(Token::Command(":"));
            tokens.push(Token::Parameter(&image[1..]));
            continue;
        }

        // Import paths
        if image.starts_with('"') && image.ends_with('"') {
            tokens.push(Token::ImportPath(&image[1..(image.len() - 1)]));
            continue;
        }

        match image {
            // Imports
            "#i" | "#it" | "#ib" => {
                tokens.push(Token::Command(image));
                continue;
            },

            _ => ()
        }

        // If the token is all hex, then it is a data value
        if image.chars().all(|c| c.is_ascii_hexdigit()) {
            tokens.push(Token::Data(image));
            continue;
        }

        println!("[Script700] {}: parse error: invalid token '{}'", context, image);
        tokens.push(Token::Invalid(image));
    }

    tokens
}

fn tokenize_data_area<'i, 'a: 'i, I: Iterator<Item = (usize, &'a str)>>(source_lines: &'i mut I, file: &'a str, is_import: bool) -> Vec<(Token<'a>, Arc<ScriptContext>)> {
    let mut result: Vec<(Token<'a>, Arc<ScriptContext>)> = Vec::new();

    loop {
        let (line_number, source_line) = match source_lines.next() {
            Some(line) => line,
            None => break
        };
        let context = Arc::new(ScriptContext::new(file, line_number, is_import));
        let line = tokenize_data_line(source_line, context.clone());
        for token in line {
            result.push((token, context.clone()));
        }
    }

    result
}

#[derive(Debug, Clone)]
pub struct TokenizeResult<'a> {
    pub script_area: Vec<(Token<'a>, Arc<ScriptContext>)>,
    pub extended_area: Vec<(Token<'a>, Arc<ScriptContext>)>,
    pub data_area: Vec<(Token<'a>, Arc<ScriptContext>)>
}

pub fn tokenize<'a>(source: &'a str, file: &'a str) -> TokenizeResult<'a> {
    let mut source_lines = source.lines().enumerate();

    TokenizeResult {
        script_area: tokenize_script_area(&mut source_lines, file, false),
        extended_area: tokenize_extended_area(&mut source_lines, file, false),
        data_area: tokenize_data_area(&mut source_lines, file, false)
    }
}

pub fn tokenize_script_import<'a>(source: &'a str, file: &'a str) -> TokenizeResult<'a> {
    let mut source_lines = source.lines().enumerate();
    TokenizeResult {
        script_area: tokenize_script_area(&mut source_lines, file, true),
        extended_area: vec![],
        data_area: vec![]
    }
}

pub fn tokenize_data_text_import<'a>(source: &'a str, file: &'a str) -> TokenizeResult<'a> {
    let mut source_lines = source.lines().enumerate();
    TokenizeResult {
        script_area: vec![],
        extended_area: vec![],
        data_area: tokenize_data_area(&mut source_lines, file, true)
    }
}
