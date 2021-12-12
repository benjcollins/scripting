use std::{fs, io::{stdin, stdout, Write}};

use parser::Parser;
use vm::VirtualMachine;

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
        match Parser::parse(&source) {
            Ok(funcs) => {
                for (i, func) in funcs.iter().enumerate() {
                    println!("func{}", i);
                    println!("{}\n", func)
                }
                VirtualMachine::run(&funcs, funcs.last().unwrap());
                source.clear();
                print!(">>> ");
                stdout().flush().unwrap();
            }
            Err(_) => {
                print!("... ");
                stdout().flush().unwrap();
            }
        }
    }
}

fn main() {
    let source = fs::read_to_string("example.txt").unwrap();
    let funcs = Parser::parse(&source).unwrap();
    for (i, func) in funcs.iter().enumerate() {
        println!("func{} - {:?}", i, func.closure_scope);
        println!("{}", func)
    }
    VirtualMachine::run(&funcs, funcs.last().unwrap());

    // repl()
}