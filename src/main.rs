#![feature(unsize)]
#![feature(coerce_unsized)]

use std::{fs, io::{stdin, stdout, Write}};

use parser::Parser;
use vm::VirtualMachine;

use crate::{parser::ParseError, func::FuncSrc};

mod lexer;
mod token;
mod parser;
mod opcode;
mod vm;
mod heap;
mod list;
mod func;

fn repl() {
    print!(">>> ");
    stdout().flush().unwrap();
    let mut source = String::new();
    let mut funcs = vec![];
    let mut symbols = vec![];
    let mut last_scope = vec![];
    let mut stack = vec![];
    loop {
        stdin().read_line(&mut source).unwrap();
        let entry_func = funcs.len();
        match Parser::parse(&source, None, &mut funcs, &mut symbols, last_scope.clone()) {
            Ok(final_scope) => {
                VirtualMachine::run(&funcs, entry_func, &symbols, &mut stack);
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

fn run_file(path: &str, disassemble: bool) {
    let source = fs::read_to_string(path).unwrap();
    let mut funcs = vec![];
    let mut symbols = vec![];
    match Parser::parse(&source, Some(path), &mut funcs, &mut symbols, vec![]) {
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
        for (i, func) in funcs.iter().enumerate() {
            println!("func{} - {:?}", i, func.closure_scope);
            println!("{}", FuncSrc::new(func, &symbols))
        }
    }
    let mut stack = vec![];
    VirtualMachine::run(&funcs, funcs.len() - 1, &symbols, &mut stack);
}

fn main() {
    repl();
    // run_file("example.txt", true)
}