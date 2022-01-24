use std::{cell::RefCell, mem::size_of, fmt::Display, convert::TryInto, iter::FromIterator};

use crate::{opcode::Opcode, parser::Symbol};

#[derive(Debug, Clone)]
pub struct FuncBuilder<'src, 'outer> {
    source: &'src str,
    bytecode: Vec<u8>,
    param_count: u8,
    closure_scope: RefCell<Vec<ClosureValue>>,
    pub scope: Vec<Symbol>,
    outer: Option<&'outer FuncBuilder<'src, 'outer>>,
}

#[derive(Debug, Clone, Default)]
pub struct Func {
    pub bytecode: Vec<u8>,
    pub param_count: u8,
    pub closure_scope: Vec<ClosureValue>,
    pub param_names: Vec<Symbol>,
}

#[derive(Debug, Clone, Copy)]
pub enum ClosureValue {
    Outer(u8),
    Stack(u8),
}

#[derive(Debug, Clone, Copy)]
pub struct Jump {
    offset: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct JumpTarget {
    offset: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum Variable {
    Stack(u8),
    Closure(u8),
}

impl<'src, 'outer> FuncBuilder<'src, 'outer> {
    pub fn new(source: &'src str, params: Vec<Symbol>) -> FuncBuilder<'src, 'outer> {
        FuncBuilder {
            source,
            bytecode: vec![],
            param_count: params.len() as u8,
            scope: params,
            closure_scope: RefCell::new(vec![]),
            outer: None,
        }
    }
    pub fn new_child(&self) -> FuncBuilder<'src, '_> {
        FuncBuilder {
            source: self.source,
            bytecode: vec![],
            param_count: 0,
            scope: vec![Symbol(0)],
            closure_scope: RefCell::new(vec![]),
            outer: Some(self),
        }
    }
    pub fn push_bytes(&mut self, bytes: &[u8]) {
        self.bytecode.extend(bytes)
    }
    pub fn resolve_stack_var(&self, symbol: Symbol) -> Option<u8> {
        self.scope.iter()
            .position(|var_symbol| *var_symbol == symbol)
            .map(|offset| offset as u8)
    }
    pub fn resolve_closure_var(&self, symbol: Symbol) -> Option<u8> {
        for i in 0..self.closure_scope.borrow().len() {
            if self.closure_var_symbol(i as u8) == symbol {
                return Some(i as u8)
            }
        }
        let outer = match self.outer {
            Some(outer) => outer,
            None => return None,
        };
        let closure_var = if let Some(index) = outer.resolve_stack_var(symbol) {
            ClosureValue::Stack(index)
        } else if let Some(index) = outer.resolve_closure_var(symbol) {
            ClosureValue::Outer(index)
        } else {
            return None
        };
        let index = self.closure_scope.borrow().len();
        self.closure_scope.borrow_mut().push(closure_var);
        Some(index as u8)
    }
    pub fn closure_var_symbol(&self, index: u8) -> Symbol {
        let outer = self.outer.unwrap();
        match self.closure_scope.borrow()[index as usize] {
            ClosureValue::Stack(index) => outer.scope[index as usize],
            ClosureValue::Outer(index) => outer.closure_var_symbol(index),
        }
    }
    pub fn resolve_var(&mut self, symbol: Symbol) -> Option<Variable> {
        if let Some(index) = self.resolve_stack_var(symbol) {
            Some(Variable::Stack(index))
        } else if let Some(index) = self.resolve_closure_var(symbol) {
            Some(Variable::Closure(index))
        } else {
            None
        }
    }
    pub fn push_var(&mut self, var: Variable) {
        match var {
            Variable::Stack(offset) => self.bytecode.extend([Opcode::PushLoad.into(), offset]),
            Variable::Closure(index) => self.bytecode.extend([Opcode::PushClosureLoad.into(), index]),
        }
    }
    pub fn pop_var(&mut self, var: Variable) {
        match var {
            Variable::Stack(offset) => self.bytecode.extend([Opcode::PopStore.into(), offset]),
            Variable::Closure(index) => self.bytecode.extend([Opcode::PopClosureStore.into(), index]),
        }
    }
    pub fn define_var(&mut self, symbol: Symbol) {
        self.scope.push(symbol);
    }
    pub fn define_param(&mut self, symbol: Symbol) {
        self.define_var(symbol);
        self.param_count += 1;
    }
    pub fn stack_size(&self) -> u8 {
        self.scope.len() as u8
    }
    pub fn free_vars(&mut self, n: u8) {
        self.scope.truncate(self.scope.len() - n as usize);
        self.bytecode.extend([Opcode::Drop.into(), n]);
    }
    pub fn push_jump(&mut self) -> Jump {
        self.bytecode.push(Opcode::Jump.into());
        let offset = self.bytecode.len() as u32;
        self.bytecode.extend(0u32.to_be_bytes());
        Jump { offset }
    }
    pub fn push_jump_if_not(&mut self) -> Jump {
        self.bytecode.push(Opcode::JumpIfNot.into());
        let offset = self.bytecode.len() as u32;
        self.bytecode.extend(0u32.to_be_bytes());
        Jump { offset }
    }
    pub fn create_jump_target(&mut self) -> JumpTarget {
        JumpTarget { offset: self.bytecode.len() as u32 }
    }
    pub fn connect_jump(&mut self, jump: Jump, target: &JumpTarget) {
        self.bytecode[jump.offset as usize..jump.offset as usize + size_of::<u32>()].copy_from_slice(&target.offset.to_be_bytes());
    }
    pub fn build(self) -> Func {
        Func {
            bytecode: self.bytecode,
            param_count: self.param_count,
            closure_scope: self.closure_scope.take(),
            param_names: Vec::from_iter(self.scope[1..self.param_count as usize + 1].iter().copied()),
        }
    }
}

struct Reader<'bytecode> {
    bytecode: &'bytecode [u8],
    offset: usize,
}

impl<'bytecode> Reader<'bytecode> {
    fn take_bytes(&mut self, n: usize) -> &'bytecode [u8] {
        let bytes = &self.bytecode[self.offset..self.offset + n];
        self.offset += n;
        bytes
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DispFunc<'a> {
    symbols: &'a [String],
    func: &'a Func,
}

impl<'a> DispFunc<'a> {
    pub fn new(func: &'a Func, symbols: &'a [String]) -> DispFunc<'a> {
        DispFunc { func, symbols }
    }
}

impl<'a> Display for DispFunc<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut reader = Reader { bytecode: &self.func.bytecode, offset: 0 };

        while reader.offset < self.func.bytecode.len() {
            write!(f, "{:>5} : ", reader.offset)?;
            let opcode: Opcode = reader.take_bytes(1)[0].try_into().unwrap();
            write!(f, "{:?} ", opcode)?;

            match opcode {
                Opcode::Add | Opcode::Subtract | Opcode::Multiply | Opcode::Divide | Opcode::Modulus |
                Opcode::Equal | Opcode::NotEqual | Opcode::Less | Opcode::Greater | Opcode::LessOrEqual | Opcode::GreaterOrEqual |
                Opcode::PushTrue | Opcode::PushFalse | Opcode::PushNone | Opcode::PopPrint |
                Opcode::Return | Opcode::Finish => writeln!(f, ""),

                Opcode::PushInt => writeln!(f, "{}", i64::from_be_bytes(reader.take_bytes(size_of::<i64>()).try_into().unwrap())),
                Opcode::PushFloat => writeln!(f, "{}", f64::from_be_bytes(reader.take_bytes(size_of::<f64>()).try_into().unwrap())),
                Opcode::PushLoad | Opcode::PopStore => {
                    writeln!(f, "{}", reader.take_bytes(1)[0])
                }
                Opcode::PushClosureLoad | Opcode::PopClosureStore |
                Opcode::PushPropLoad | Opcode::PopPropStore |
                Opcode::Drop | Opcode::Call => writeln!(f, "{}", reader.take_bytes(1)[0]),
                Opcode::Jump | Opcode::JumpIfNot | Opcode::PushList => writeln!(f, "{}", u32::from_be_bytes(reader.take_bytes(size_of::<u32>()).try_into().unwrap())),
                Opcode::PushFunc => writeln!(f, "func{}", u32::from_be_bytes(reader.take_bytes(size_of::<u32>()).try_into().unwrap())),
            }?;
        }

        Ok(())
    }
}