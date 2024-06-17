use std::sync::Arc;
use crate::script700::lexer::{Token, TokenizeResult, tokenize_data_text_import};
use crate::script700::context::{ImportContext, ScriptContext};
use super::script_area::parse_import_path;

pub fn decode_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }

    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect::<Result<Vec<u8>, _>>()
        .ok()
}

#[derive(Debug, Clone)]
pub struct ParsedDataArea {
    pub data: Vec<u8>,
    pub labels: Vec<(u16, Arc<ScriptContext>, usize)>
}

impl ParsedDataArea {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            labels: Vec::new()
        }
    }
}

fn parse_label(s: &str) -> Option<u16> {
    if !s.starts_with(":") {
        return None;
    }

    s[1..].parse::<u16>().ok().map(|label| label % 1024)
}

pub fn parse<'a>(tokenize_result: &'a TokenizeResult<'a>, import_context: ImportContext) -> ParsedDataArea {
    let mut result = ParsedDataArea::new();
    let mut hex_data = String::new();
    let mut token_iter = tokenize_result.data_area.iter();

    loop {
        match token_iter.next() {
            Some((Token::Command(command), context)) => {
                match *command {
                    "#i" | "#it" => {
                        let path = match parse_import_path(&mut token_iter) {
                            Some(path) => path,
                            None => continue
                        };
                        let source = match import_context.resolve_text_import(&path, context.clone()) {
                            Some(source) => source,
                            None => continue
                        };
                        let imported_data = tokenize_data_text_import(&source, &path);
                        let parsed_imported_data = parse(&imported_data, ImportContext::Import);
                        let label_offset = result.data.len();
                        result.data.extend_from_slice(&parsed_imported_data.data);
                        for (label, label_context, offset) in parsed_imported_data.labels {
                            result.labels.push((label, label_context, offset + label_offset));
                        }
                        continue;
                    },
                    "#ib" => {
                        let path = match parse_import_path(&mut token_iter) {
                            Some(path) => path,
                            None => continue
                        };
                        let data = match import_context.resolve_binary_import(&path, context.clone()) {
                            Some(data) => data,
                            None => continue
                        };
                        result.data.extend_from_slice(&data);
                        continue;
                    },

                    _ => ()
                };

                if let Some(label) = parse_label(*command) {
                    if let Some(data_block) = decode_hex(&hex_data) {
                        result.data.extend(data_block);
                        result.labels.push((label, context.clone(), result.data.len()));
                    } else {
                        println!("[Script700] {}: warning: malformed hex data", context);
                    }
                    hex_data.clear();
                }
            },
            Some((Token::Data(hex), _)) => {
                hex_data.push_str(*hex);
            }

            _ => break
        };
    }

    if let Some(data_block) = decode_hex(&hex_data) {
        result.data.extend(data_block);
    } else if let Some((_, last_context)) = tokenize_result.data_area.last() {
        println!("[Script700] {}: warning: malformed hex data", last_context);
    }

    result
}
