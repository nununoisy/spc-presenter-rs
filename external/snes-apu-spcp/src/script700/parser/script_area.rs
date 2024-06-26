use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;
use crate::script700::lexer::{Token, TokenizeResult, tokenize_script_import};
use crate::script700::context::{ImportContext, ScriptContext};
use super::expect_token;

#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub enum MemoryAccessWidth {
    #[default]
    Byte,
    Word,
    DWord
}

impl MemoryAccessWidth {
    pub fn width(&self) -> u32 {
        match self {
            MemoryAccessWidth::Byte => 1,
            MemoryAccessWidth::Word => 2,
            MemoryAccessWidth::DWord => 4
        }
    }
}

fn parse_numeric_constant(s: &str) -> Option<u32> {
    if s.to_lowercase().starts_with("0x") {
        u32::from_str_radix(&s[2..], 16)
    } else if s.starts_with("$") {
        u32::from_str_radix(&s[1..], 16)
    } else {
        u32::from_str_radix(s, 10)
    }
        .ok()
}

#[derive(Copy, Clone, PartialEq)]
pub enum ParameterValue<const MAX: u32> {
    Literal(u32),
    Dynamic(bool)
}

impl ParameterValue<{ u32::MAX }> {
    pub fn parse(s: &str, second_dynamic_parameter: bool) -> Option<Self> {
        if s == "?" {
            return Some(ParameterValue::Dynamic(second_dynamic_parameter));
        }

        parse_numeric_constant(s).map(|value| ParameterValue::Literal(value))
    }
}

impl<const MAX: u32> ParameterValue<MAX> {
    pub fn get(&self, cmp1: u32, cmp2: u32) -> u32 {
        match self {
            ParameterValue::Literal(value) => *value % MAX,
            ParameterValue::Dynamic(false) => cmp1 % MAX,
            ParameterValue::Dynamic(true) => cmp2 % MAX
        }
    }

    pub fn cast<const NEW_MAX: u32>(self) -> ParameterValue<NEW_MAX> {
        match self {
            ParameterValue::Literal(value) => ParameterValue::Literal(value),
            ParameterValue::Dynamic(second) => ParameterValue::Dynamic(second)
        }
    }
}

impl<const MAX: u32> Display for ParameterValue<MAX> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            ParameterValue::Literal(value) => write!(f, "{}", value),
            ParameterValue::Dynamic(false) => write!(f, "cmp1"),
            ParameterValue::Dynamic(true) => write!(f, "cmp2")
        }
    }
}

impl<const MAX: u32> Debug for ParameterValue<MAX> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "ParameterValue<{}>({})", MAX, self)
        } else {
            write!(f, "{}", self)
        }
    }
}

pub fn parse_nonparametric_literal<'i, 'a: 'i, I: Iterator<Item = &'a (Token<'a>, Arc<ScriptContext>)>>(token_iter: &'i mut I, max: u32) -> Option<u32> {
    let (image, context) = expect_token!(token_iter, Parameter);

    match parse_numeric_constant(image) {
        Some(value) => Some(value % max),
        None => {
            println!("[Script700] {}: parse error: invalid number '{}'", context, image);
            None
        }
    }
}

pub fn parse_import_path<'i, 'a: 'i, I: Iterator<Item = &'a (Token<'a>, Arc<ScriptContext>)>>(token_iter: &'i mut I) -> Option<String> {
    let (image, context) = expect_token!(token_iter, ImportPath);
    let mut result = String::new();

    let mut char_iter = image.chars();
    loop {
        match char_iter.next() {
            Some('\\') => match char_iter.next() {
                Some(c) if c == '"' || c == '\\' => result.push(c),
                Some(other) => {
                    println!("[Script700] {}: parse error: invalid escape sequence '\\{}'", context, other);
                    return None;
                },
                None => {
                    println!("[Script700] {}: parse error: invalid import path", context);
                    return None;
                }
            },
            Some('"') => {
                println!("[Script700] {}: parse error: invalid import path", context);
                return None;
            }
            Some(c) => result.push(c),
            None => break
        }
    }

    Some(result)
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Parameter {
    Number(ParameterValue<{ u32::MAX }>),
    InputPort(ParameterValue<4>),
    OutputPort(ParameterValue<4>),
    WorkingMemory(ParameterValue<8>),
    ARAM(MemoryAccessWidth, ParameterValue<{ u16::MAX as _ }>),
    IplROM(ParameterValue<64>),
    DataArea(MemoryAccessWidth, ParameterValue<{ u32::MAX }>),
    Label(ParameterValue<1024>)
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ParameterClass {
    Source,
    Destination,
    Numeric,
    BranchTarget
}

impl Parameter {
    pub fn parse<'i, 'a: 'i, I: Iterator<Item = &'a (Token<'a>, Arc<ScriptContext>)>>(token_iter: &'i mut I, class: ParameterClass, second_dynamic_parameter: bool) -> Option<Self> {
        let (image, context) = expect_token!(token_iter, Parameter);

        let value_start_index = image.chars().position(|c| c.is_numeric() || c == '?' || c == '$')?;
        let value = ParameterValue::parse(&image[value_start_index..], second_dynamic_parameter)?;

        let result = match image[..value_start_index].to_lowercase().as_str() {
            "#" => Parameter::Number(value),
            "" if class == ParameterClass::Numeric || class == ParameterClass::BranchTarget => Parameter::Number(value),

            "i" => Parameter::InputPort(value.cast()),
            "" if class == ParameterClass::Destination => Parameter::InputPort(value.cast()),

            "o" => Parameter::OutputPort(value.cast()),
            "" if class == ParameterClass::Source => Parameter::OutputPort(value.cast()),

            "w" => Parameter::WorkingMemory(value.cast()),
            "r" | "rb" => Parameter::ARAM(MemoryAccessWidth::Byte, value.cast()),
            "rw" => Parameter::ARAM(MemoryAccessWidth::Word, value.cast()),
            "rd" => Parameter::ARAM(MemoryAccessWidth::DWord, value.cast()),
            "x" => Parameter::IplROM(value.cast()),
            "d" | "db" => Parameter::DataArea(MemoryAccessWidth::Byte, value.cast()),
            "dw" => Parameter::DataArea(MemoryAccessWidth::Word, value.cast()),
            "dd" => Parameter::DataArea(MemoryAccessWidth::DWord, value.cast()),
            "l" => Parameter::Label(value.cast()),

            _ => return None
        };

        match (class, &result) {
            // Destination cannot be a literal or label
            (ParameterClass::Destination, Parameter::Number(value)) => {
                println!("[Script700] {}: parse error: '#{:?}' cannot be a destination parameter", context, value);
                return None
            },
            (ParameterClass::Destination, Parameter::Label(value)) => {
                println!("[Script700] {}: parse error: 'l{:?}' cannot be a destination parameter", context, value);
                return None
            },

            // Branch target must be a number, label, or working memory reference
            (ParameterClass::BranchTarget, Parameter::Number(_)) => (),
            (ParameterClass::BranchTarget, Parameter::Label(_)) => (),
            (ParameterClass::BranchTarget, Parameter::WorkingMemory(_)) => (),
            (ParameterClass::BranchTarget, _) => return None,

            _ => ()
        }

        Some(result)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
    UnsignedDivide,
    Remainder,
    Modulus,
    And,
    Or,
    Xor,
    ShiftLeft,
    ShiftRightArithmetic,
    ShiftRightLogical,
    Not
}

impl Operation {
    pub fn parse<'i, 'a: 'i, I: Iterator<Item = &'a (Token<'a>, Arc<ScriptContext>)>>(token_iter: &'i mut I) -> Option<Self> {
        let (image, context) = expect_token!(token_iter, Operation);

        let result = match image {
            "+" => Operation::Add,
            "-" => Operation::Subtract,
            "*" => Operation::Multiply,
            "/" => Operation::Divide,
            "\\" => Operation::UnsignedDivide,
            "%" => Operation::Remainder,
            "$" => Operation::Modulus,
            "&" => Operation::And,
            "|" => Operation::Or,
            "^" => Operation::Xor,
            "<" => Operation::ShiftLeft,
            "_" => Operation::ShiftRightArithmetic,
            ">" => Operation::ShiftRightLogical,
            "!" => Operation::Not,

            _ => {
                println!("[Script700] {}: parse error: invalid operation '{}'", context, image);
                return None
            }
        };

        Some(result)
    }

    pub fn from_short_command(command_token_image: &str) -> Self {
        match command_token_image {
            "a" => Operation::Add,
            "s" => Operation::Subtract,
            "u" => Operation::Multiply,
            "d" => Operation::Divide,

            _ => unreachable!()
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Condition {
    Always,
    Equal,
    NotEqual,
    GreaterOrEqual,
    LessOrEqual,
    GreaterThan,
    LessThan,
    CarryClear,
    Lower,
    Higher,
    CarrySet
}

impl Condition {
    pub fn parse(branch_token_image: &str) -> Option<Self> {
        Some(match branch_token_image {
            "bra" => Condition::Always,
            "beq" => Condition::Equal,
            "bne" => Condition::NotEqual,
            "bge" => Condition::GreaterOrEqual,
            "ble" => Condition::LessOrEqual,
            "bgt" => Condition::GreaterThan,
            "blt" => Condition::LessThan,
            "bcc" => Condition::CarryClear,
            "blo" => Condition::Lower,
            "bhi" => Condition::Higher,
            "bcs" => Condition::CarrySet,

            _ => return None
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Label { label: u16 },
    Wait { cycles: Parameter },
    Move { src: Parameter, dst: Parameter },
    Compare { cmp1: Parameter, cmp2: Parameter },
    Numeric { val: Parameter, op: Operation, dst: Parameter },
    Branch { condition: Condition, target: Parameter },
    Return,
    SetCallStack { enabled: bool },
    FlushInputPorts,
    SetUnbufferedInputPorts { enabled: bool },
    WaitPort { port: Parameter },
    Quit,
    Nop,
    Import { path: String }
}

impl Command {
    pub fn parse<'i, 'a: 'i, I: Iterator<Item = &'a (Token<'a>, Arc<ScriptContext>)>>(token_iter: &'i mut I) -> Option<(Self, Arc<ScriptContext>)> {
        let (image, context) = expect_token!(token_iter, Command);

        Some(match image {
            ":" => {
                let label = parse_nonparametric_literal(token_iter, 1024)? as u16;
                (Command::Label { label }, context)
            },
            "w" => {
                let cycles = Parameter::parse(token_iter, ParameterClass::Numeric, false)?;
                (Command::Wait { cycles }, context)
            },
            "m" => {
                let src = Parameter::parse(token_iter, ParameterClass::Source, false)?;
                let dst = Parameter::parse(token_iter, ParameterClass::Destination, true)?;
                (Command::Move { src, dst }, context)
            },
            "c" => {
                let cmp1 = Parameter::parse(token_iter, ParameterClass::Source, false)?;
                let cmp2 = Parameter::parse(token_iter, ParameterClass::Source, true)?;
                (Command::Compare { cmp1, cmp2 }, context)
            },
            "a" | "s" | "u" | "d" => {
                let val = Parameter::parse(token_iter, ParameterClass::Source, false)?;
                let op = Operation::from_short_command(image);
                let dst = Parameter::parse(token_iter, ParameterClass::Destination, true)?;
                (Command::Numeric { val, op, dst }, context)
            },
            "n" => {
                let val = Parameter::parse(token_iter, ParameterClass::Source, false)?;
                let op = Operation::parse(token_iter)?;
                let dst = Parameter::parse(token_iter, ParameterClass::Destination, true)?;
                (Command::Numeric { val, op, dst }, context)
            },
            "r" => (Command::Return, context),
            "r0" => (Command::SetCallStack { enabled: false }, context),
            "r1" => (Command::SetCallStack { enabled: true }, context),
            "f" => (Command::FlushInputPorts, context),
            "f0" => (Command::SetUnbufferedInputPorts { enabled: false }, context),
            "f1" => (Command::SetUnbufferedInputPorts { enabled: true }, context),
            "wi" => {
                let port = Parameter::InputPort(ParameterValue::Literal(parse_nonparametric_literal(token_iter, 4)?));
                (Command::WaitPort { port }, context)
            },
            "wo" => {
                let port = Parameter::OutputPort(ParameterValue::Literal(parse_nonparametric_literal(token_iter, 4)?));
                (Command::WaitPort { port }, context)
            },
            "q" | "e" => (Command::Quit, context),
            "nop" => (Command::Nop, context),
            "#i" | "#it" => {
                let path = parse_import_path(token_iter)?;
                (Command::Import { path }, context)
            },
            special => {
                // Branch
                if let Some(condition) = Condition::parse(special) {
                    let target = Parameter::parse(token_iter, ParameterClass::BranchTarget, false)?;
                    (Command::Branch { condition, target }, context)
                } else {
                    println!("[Script700] {}: parse error: unknown command '{}'", context, special);
                    return None
                }
            }
        })
    }
}

pub type ScriptAst = Vec<(Command, Arc<ScriptContext>)>;

pub fn parse<'a>(tokenize_result: &'a TokenizeResult<'a>, import_context: ImportContext) -> ScriptAst {
    let mut result = ScriptAst::new();
    let mut token_iter = tokenize_result.script_area.iter();

    loop {
        match Command::parse(&mut token_iter) {
            Some((Command::Import { path }, import_command_context)) => {
                let source = match import_context.resolve_text_import(&path, import_command_context.clone()) {
                    Some(source) => source,
                    None => continue
                };
                let imported_script = tokenize_script_import(&source, &path);
                let parsed_imported_script = parse(&imported_script, ImportContext::Import);
                result.extend_from_slice(&parsed_imported_script);
            }
            Some((command, context)) => result.push((command, context)),
            _ => break
        }
    }

    result
}
