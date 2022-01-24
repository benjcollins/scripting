#![feature(unsize)]
#![feature(coerce_unsized)]

use std::{fs, io::{stdin, stdout, Write}};

use heap::Heap;
use parser::Parser;
use vm::VirtualMachine;

use crate::{parser::{ParseError, Program}, func::DispFunc, vm::Value};

mod lexer;
mod token;
mod parser;
mod opcode;
mod vm;
mod heap;
mod list;
mod func;
mod symbols;

fn _repl() {
    print!(">>> ");
    stdout().flush().unwrap();
    let mut source = String::new();
    let mut program = Program::new();
    let mut last_scope = vec![symbols::RETURN];
    let mut stack = vec![Value::None];
    let mut heap = Heap::new();
    loop {
        stdin().read_line(&mut source).unwrap();
        let entry_func = program.funcs.len();
        match Parser::parse(&source, None, &mut program, last_scope.clone()) {
            Ok(final_scope) => {
                VirtualMachine::run(&program, entry_func, &mut stack, &mut heap);
                last_scope = final_scope;
                source.clear();
                print!(">>> ");
            }
            Err(ParseError::EndOfInput) => {
                print!("... ");
            }
            Err(ParseError::InvalidInput(err)) => {
                println!("{}", err);
                source.clear();
                print!(">>> ");
            }
        }
        stdout().flush().unwrap();
    }
}

fn _run_file(path: &str, disassemble: bool) {
    let source = fs::read_to_string(path).unwrap();
    let mut program = Program::new();
    match Parser::parse(&source, Some(path), &mut program, vec![symbols::RETURN]) {
        Ok(_) => (),
        Err(ParseError::EndOfInput) => {
            println!("unexpected end of input");
            return
        }
        Err(ParseError::InvalidInput(err)) => {
            println!("{}", err);
            return
        }
    };
    if disassemble {
        for func in program.funcs.iter() {
            println!("{}", DispFunc::new(func, &program.symbols))
        }
    }
    let mut stack = vec![Value::None];
    let mut heap = Heap::new();
    VirtualMachine::run(&program, 0, &mut stack, &mut heap);
}

fn main() {
    _repl();
    // _run_file("example.txt", true)
}