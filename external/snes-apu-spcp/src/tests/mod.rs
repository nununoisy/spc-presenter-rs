use std::fs;
use std::path::PathBuf;

#[test]
fn script700_tokenizer() {
    use crate::script700::tokenizer::TokenizerAdapter;

    const TEST_LINE: &'static str = "m #1\tw0 :0 bra 1 :1 c   w1\t\t#? #it \"test.700\" #it  \"test 2.700\" #ib \"test 3\\\"   .700";
    let tokenize_result: Vec<_> = TEST_LINE.tokenize().collect();

    assert_eq!(tokenize_result, vec![
        "m", "#1", "w0",
        ":0",
        "bra", "1",
        ":1",
        "c", "w1", "#?",
        "#it", "\"test.700\"",
        "#it", "\"test 2.700\"",
        "#ib", "\"test 3\\\"   .700"
    ]);
}

#[test]
fn script700_lexer() {
    use crate::script700::lexer::*;

    const TEST_SCRIPT: &'static str = include_str!("drums.700");
    let tokenize_result = tokenize(TEST_SCRIPT, "drums.700");

    let script_tokens: Vec<_> = tokenize_result.script_area
        .iter()
        .map(|(token, _)| token)
        .copied()
        .collect();
    assert_eq!(script_tokens, vec![
        Token::Command("m"), Token::Parameter("#57000"), Token::Parameter("w0"),
        Token::Command("r0"),
        Token::Command(":"), Token::Parameter("0"),
        Token::Command("w"), Token::Parameter("#2048"),
        Token::Command("s"), Token::Parameter("#1"), Token::Parameter("w0"),
        Token::Command("c"), Token::Parameter("#0"), Token::Parameter("w0"),
        Token::Command("bne"), Token::Parameter("0"),
        Token::Command("m"), Token::Parameter("#0x02"), Token::Parameter("i1"),
        Token::Command("q"),
        Token::Command("e")
    ]);

    let extended_tokens: Vec<_> = tokenize_result.extended_area
        .iter()
        .map(|(token, _)| token)
        .copied()
        .collect();
    assert_eq!(extended_tokens, vec![
        Token::Command("::"),
        Token::Command("e")
    ]);

    let data_tokens: Vec<_> = tokenize_result.data_area
        .iter()
        .map(|(token, _)| token)
        .copied()
        .collect();
    assert_eq!(data_tokens, vec![
        Token::Data("3E7F8888"),
        Token::Command(":"), Token::Parameter("12"),
        Token::Data("87C"),
        Token::Data("8"),
        Token::Data("2E"),
        Token::Data("39")
    ])
}

#[test]
fn script700_script_area_parser() {
    use crate::script700::lexer::tokenize;
    use crate::script700::context::ImportContext;
    use crate::script700::parser::script_area::*;

    const TEST_SCRIPT: &'static str = include_str!("drums2.700");
    let tokenize_result = tokenize(TEST_SCRIPT, "drums2.700");

    let import_root_path = PathBuf::from_iter([env!("CARGO_MANIFEST_DIR"), file!(), ".."].iter());
    let script_ast = parse(&tokenize_result, ImportContext::Root(import_root_path));

    let commands: Vec<_> = script_ast
        .iter()
        .map(|(command, _)| command)
        .cloned()
        .collect();
    assert_eq!(commands, vec![
        Command::Move {
            src: Parameter::Number(ParameterValue::Literal(57000)),
            dst: Parameter::WorkingMemory(ParameterValue::Literal(0))
        },
        Command::Branch {
            condition: Condition::Always,
            target: Parameter::Number(ParameterValue::Literal(100))
        },
        Command::Compare {
            cmp1: Parameter::Number(ParameterValue::Literal(2)),
            cmp2: Parameter::Number(ParameterValue::Literal(1))
        },
        Command::Move {
            src: Parameter::Number(ParameterValue::Dynamic(false)),
            dst: Parameter::InputPort(ParameterValue::Dynamic(true)),
        },
        Command::Quit,
        // begin imported file
        Command::Label { label: 100 },
        Command::SetCallStack { enabled: false },
        Command::Label { label: 0 },
        Command::Wait {
            cycles: Parameter::Number(ParameterValue::Literal(2048))
        },
        Command::Numeric {
            val: Parameter::Number(ParameterValue::Literal(1)),
            op: Operation::Subtract,
            dst: Parameter::WorkingMemory(ParameterValue::Literal(0))
        },
        Command::Compare {
            cmp1: Parameter::Number(ParameterValue::Literal(0)),
            cmp2: Parameter::WorkingMemory(ParameterValue::Literal(0))
        },
        Command::Branch {
            condition: Condition::NotEqual,
            target: Parameter::Number(ParameterValue::Literal(0))
        },
        Command::Return,
        // end imported file
        Command::Quit
    ]);
}

#[test]
fn script700_data_area_parser() {
    use crate::script700::lexer::tokenize;
    use crate::script700::context::ImportContext;
    use crate::script700::parser::data_area::*;

    const TEST_SCRIPT: &'static str = include_str!("drums2.700");
    let tokenize_result = tokenize(TEST_SCRIPT, "drums2.700");

    let import_root_path = PathBuf::from_iter([env!("CARGO_MANIFEST_DIR"), file!(), ".."].iter());
    let parsed_data_area = parse(&tokenize_result, ImportContext::Root(import_root_path));

    assert_eq!(parsed_data_area.data.clone(), vec![
        0x3e, 0x7f, 0x88, 0x88, 0x87, 0xc8, 0x2e, 0x39,
        0x33, 0x45, 0x37, 0x46, 0x38, 0x38, 0x38, 0x38, 0x20, 0x3A, 0x31, 0x32, 0x20, 0x38, 0x37, 0x43, 0x20, 0x38, 0x20, 0x32, 0x45, 0x20, 0x33, 0x39
    ]);
    let labels: Vec<_> = parsed_data_area.labels
        .iter()
        .map(|(label, _, offset)| (*label, *offset))
        .collect();
    assert_eq!(labels, vec![
        (12, 4),
        (34, 8)
    ]);
}
