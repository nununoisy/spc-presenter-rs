use crate::script700::lexer::tokenize;
use crate::script700::parser;
use crate::script700::context::ImportContext;
use std::fs;
use std::path::PathBuf;

fn script700_test(file: &str) {
    println!("--- {} ---", file);

    let path = PathBuf::from_iter([env!("CARGO_MANIFEST_DIR"), file!(), ".."].iter());
    let source = fs::read_to_string(path.join(file)).unwrap();
    let tokenize_result = tokenize(&source, file);

    println!("--- Script Area Tokens ---");
    for (token, context) in tokenize_result.script_area.iter() {
        println!("{} :: {:?}", context, token);
    }
    println!("--- Script Area AST ---");
    let script_ast = parser::script_area::parse(&tokenize_result, ImportContext::Root(path.clone()));
    for (i, (command, context)) in script_ast.iter().enumerate() {
        println!("i{} {} :: {:?}", i, context, command);
    }

    println!("--- Extended Area Tokens ---");
    for (token, context) in tokenize_result.extended_area.iter() {
        println!("{} :: {:?}", context, token);
    }

    println!("--- Data Area Tokens ---");
    for (token, context) in tokenize_result.data_area.iter() {
        println!("{} :: {:?}", context, token);
    }

    println!("--- Data Area Parse Results ---");
    let data_area = parser::data_area::parse(&tokenize_result, ImportContext::Root(path.clone()));
    println!("{:x?}", data_area.data);
    for (label, context, ptr) in data_area.labels.iter() {
        println!("{} :{} -> {}", context, label, ptr);
    }

    println!("------------------");
}

#[test]
fn script700_drums() {
    script700_test("drums.700");
}

#[test]
fn script700_c700() {
    script700_test("c700.700");
}

#[test]
fn script700_import() {
    script700_test("drums2.700");
}
