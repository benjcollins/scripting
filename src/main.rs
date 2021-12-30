#![feature(unsize)]
#![feature(coerce_unsized)]

use std::{fs, io::{stdin, stdout, Write}};

use parser::Parser;
use vm::VirtualMachine;

use crate::parser::ParseError;

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
    loop {
        stdin().read_line(&mut source).unwrap();
        match Parser::parse(&source, None) {
            Ok((funcs, props)) => {
                VirtualMachine::run(&funcs, funcs.last().unwrap(), &props);
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
    let (funcs, props) = match Parser::parse(&source, Some(path)) {
        Ok(x) => x,
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
            println!("{}", func)
        }
    }
    VirtualMachine::run(&funcs, funcs.last().unwrap(), &props);
}

fn main() {
    repl();
    // run_file("example.txt", true)
}