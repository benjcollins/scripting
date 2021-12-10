use std::mem::{size_of, replace};

use crate::{lexer::Lexer, opcode::Opcode, token::{Token, TokenType}};

pub struct Parser<'src> {
    lexer: Lexer<'src>,
    token: Token<'src>,

    func: Func<'src>,
    func_stack: Vec<Func<'src>>,

    finished_funcs: Vec<CompletedFunc<'src>>,
}

#[derive(Debug, Clone)]
pub struct Func<'src> {
    bytecode: Vec<u8>,
    param_count: u8,
    scope: Vec<&'src str>,
    closure_scope: Vec<ClosureVarDecl<'src>>,
}

#[derive(Debug, Clone, Copy)]
struct ClosureVarDecl<'src> {
    name: &'src str,
    val: ClosureValue,
}

#[derive(Debug, Clone, Copy)]
pub enum ClosureValue {
    Outer(u8),
    Stack(u8),
}

#[derive(Debug)]
pub struct CompletedFunc<'src> {
    pub bytecode: Vec<u8>,
    pub param_count: u8,
    pub scope: Vec<&'src str>,
    pub closure_scope: Vec<ClosureValue>,
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
enum Precedence {
    Product,
    Sum,
    Relational,
    Equality,
    Top,
}

enum Assoc {
    Left,
    Right,
}

pub struct Jump {
    offset: u32,
}

pub struct JumpTarget {
    offset: u32,
}

#[derive(Debug, Clone, Copy)]
enum Variable {
    Stack(u8),
    Closure(u8),
}

enum ParsedInfixOp {
    Complete {
        opcode: Opcode,
        prec: Precedence,
    },
    Incomplete {
        prec: Precedence,
        token: TokenType,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct MissingInput {}

impl ParsedInfixOp {
    fn new(prec: Precedence, token: TokenType) -> ParsedInfixOp {
        ParsedInfixOp::Incomplete { prec, token }
    }
    fn parse_infix_op(self, op_token: TokenType, prec: Precedence, opcode: Opcode, assoc: Assoc) -> ParsedInfixOp {
        match self {
            ParsedInfixOp::Incomplete { prec: current_prec, token } => {
                if match assoc {
                    Assoc::Left => prec <= current_prec,
                    Assoc::Right => prec < current_prec,
                } && token == op_token {
                    ParsedInfixOp::Complete { opcode, prec }
                } else {
                    ParsedInfixOp::Incomplete { prec: current_prec, token }
                }
            }
            complete => complete,
        }
    }
}

impl<'src> Func<'src> {
    fn new() -> Func<'src> {
        Func { bytecode: vec![], param_count: 0, scope: vec![""], closure_scope: vec![] }
    }
    fn resolve_var(&self, name: &'src str) -> Option<u8> {
        self.scope.iter()
            .copied().enumerate().rev()
            .find(|(_, var_name)| *var_name == name)
            .map(|(i, _)| i as u8)
    }
    fn push_jump(&mut self) -> Jump {
        self.bytecode.push(Opcode::Jump.into());
        let offset = self.bytecode.len() as u32;
        self.bytecode.extend(0u32.to_be_bytes());
        Jump { offset }
    }
    fn push_jump_if_not(&mut self) -> Jump {
        self.bytecode.push(Opcode::JumpIfNot.into());
        let offset = self.bytecode.len() as u32;
        self.bytecode.extend(0u32.to_be_bytes());
        Jump { offset }
    }
    fn create_jump_target(&mut self) -> JumpTarget {
        JumpTarget { offset: self.bytecode.len() as u32 }
    }
    fn connect_jump(&mut self, jump: Jump, target: &JumpTarget) {
        self.bytecode[jump.offset as usize..jump.offset as usize + size_of::<u32>()].copy_from_slice(&target.offset.to_be_bytes());
    }
    fn push_var(&mut self, var: Variable) {
        match var {
            Variable::Stack(offset) => self.bytecode.extend([Opcode::PushLoad.into(), offset]),
            Variable::Closure(index) => self.bytecode.extend([Opcode::PushClosureLoad.into(), index]),
        }
    }
    fn pop_var(&mut self, var: Variable) {
        match var {
            Variable::Stack(offset) => self.bytecode.extend([Opcode::PopStore.into(), offset]),
            Variable::Closure(index) => self.bytecode.extend([Opcode::PopClosureStore.into(), index]),
        }
    }
}

impl<'src> CompletedFunc<'src> {
    fn new(func: Func) -> CompletedFunc {
        CompletedFunc {
            bytecode: func.bytecode,
            param_count: func.param_count,
            scope: func.scope,
            closure_scope: func.closure_scope.iter().map(|var| var.val).collect(),
        }
    }
}

impl<'src> Parser<'src> {
    fn next(&mut self) {
        self.token = self.lexer.next_token();
    }
    fn eat(&mut self, ty: TokenType) -> bool {
        if self.token.ty == ty {
            self.next();
            true
        } else {
            false
        }
    }
    fn invalid_token(&mut self) -> Result<(), MissingInput> {
        if self.token.ty == TokenType::End {
            Err(MissingInput {})
        } else {
            panic!("{:?}", self.token)
        }
    }
    fn resolve_closure_index(&mut self, name: &'src str, func_index: usize) -> Option<ClosureValue> {
        let func = &mut self.func_stack[func_index];

        let closure_var = func.closure_scope.iter()
            .enumerate().rev()
            .find(|(_, var)| var.name == name)
            .map(|(i, _)| i as u8);
        
        if let Some(index) = closure_var {
            return Some(ClosureValue::Outer(index))
        }

        if let Some(index) = func.resolve_var(name) {
            return Some(ClosureValue::Stack(index))
        }

        if func_index == 0 {
            None
        } else {
            let closure_val = self.resolve_closure_index(name, func_index-1)?;
            let closure_index = self.func_stack[func_index].closure_scope.len();
            self.func_stack[func_index].closure_scope.push(ClosureVarDecl { name, val: closure_val });
            Some(ClosureValue::Outer(closure_index as u8))
        }
    }
    fn resolve_var(&mut self, name: &'src str) -> Variable {
        match self.func.resolve_var(name) {
            Some(offset) => Variable::Stack(offset),
            None => match self.resolve_closure_index(name, self.func_stack.len()-1) {
                Some(closure_val) => {
                    let closure_index = self.func.closure_scope.len();
                    self.func.closure_scope.push(ClosureVarDecl { name, val: closure_val });
                    Variable::Closure(closure_index as u8)
                },
                None => panic!(),
            },
        }
    }
    fn parse_call(&mut self, name: &'src str) -> Result<(), MissingInput> {
        self.next();
        self.func.bytecode.push(Opcode::PushNone.into());
        let mut arg_count = 0;
        if !self.eat(TokenType::CloseBrace) {
            loop {
                self.parse_expr()?;
                arg_count += 1;
                if !self.eat(TokenType::Comma) {
                    break
                }
            }
            if !self.eat(TokenType::CloseBrace) {
                self.invalid_token()?;
            }
        }
        let var = self.resolve_var(name);
        self.func.push_var(var);
        self.func.bytecode.extend([Opcode::Call.into(), arg_count]);
        Ok(())
    }
    fn parse_value(&mut self) -> Result<(), MissingInput> {
        match self.token.ty {
            TokenType::Ident => {
                let name = self.token.source;
                self.next();
                if self.token.ty == TokenType::OpenBrace {
                    self.parse_call(name)?;
                } else {
                    let var = self.resolve_var(name);
                    self.func.push_var(var);
                }
            }
            TokenType::Int => {
                let c: i64 = self.token.source.parse().unwrap();
                self.next();
                self.func.bytecode.push(Opcode::PushInt.into());
                self.func.bytecode.extend(c.to_be_bytes());
            }
            TokenType::True => {
                self.next();
                self.func.bytecode.push(Opcode::PushTrue.into());
            }
            TokenType::False => {
                self.next();
                self.func.bytecode.push(Opcode::PushFalse.into());
            }
            TokenType::None => {
                self.next();
                self.func.bytecode.push(Opcode::PushNone.into());
            }
            TokenType::OpenBrace => {
                self.next();
                self.parse_expr()?;
                if !self.eat(TokenType::CloseBrace) {
                    self.invalid_token()?;
                }
            }
            TokenType::OpenSquareBrace => {
                self.next();
                let mut length: u32 = 0;
                if !self.eat(TokenType::CloseSquareBrace) {
                    loop {
                        self.parse_expr()?;
                        length += 1;
                        if !self.eat(TokenType::Comma) {
                            break
                        }
                    }
                    if !self.eat(TokenType::CloseSquareBrace) {
                        self.invalid_token()?;
                    }
                }
                self.func.bytecode.push(Opcode::PushList.into());
                self.func.bytecode.extend(length.to_be_bytes());
            }
            TokenType::Func => {
                self.next();
                self.func.bytecode.push(Opcode::PushFunc.into());
                self.func.bytecode.extend((self.finished_funcs.len() as u32).to_be_bytes());
                let func_index = self.finished_funcs.len();
                self.finished_funcs.push(CompletedFunc { bytecode: vec![], closure_scope: vec![], scope: vec![], param_count: 0 });

                self.func_stack.push(replace(&mut self.func, Func::new()));

                if !self.eat(TokenType::OpenBrace) {
                    self.invalid_token()?;
                }
                if !self.eat(TokenType::CloseBrace) {
                    loop {
                        let name = self.token.source;
                        if !self.eat(TokenType::Ident) {
                            self.invalid_token()?;
                        }
                        self.func.scope.push(name);
                        self.func.param_count += 1;
                        if !self.eat(TokenType::Comma) {
                            break
                        }
                    }
                    if !self.eat(TokenType::CloseBrace) {
                        self.invalid_token()?;
                    }
                }

                if self.token.ty == TokenType::OpenCurlyBrace {
                    self.parse_block()?;
                } else {
                    self.parse_expr()?;
                    self.func.bytecode.extend([Opcode::PopStore.into(), 0]);
                }
                self.func.bytecode.push(Opcode::Return.into());

                let finished_func = replace(&mut self.func, self.func_stack.pop().unwrap());
                self.finished_funcs[func_index] = CompletedFunc::new(finished_func);
            }
            _ => self.invalid_token()?,
        };
        Ok(())
    }
    fn parse_infix(&mut self, prec: Precedence) -> Result<(), MissingInput> {
        loop {
            let parsed = ParsedInfixOp::new(prec, self.token.ty)
                .parse_infix_op(TokenType::Plus, Precedence::Sum, Opcode::Add, Assoc::Right)
                .parse_infix_op(TokenType::Minus, Precedence::Sum, Opcode::Subtract, Assoc::Right)
                .parse_infix_op(TokenType::Multiply, Precedence::Product, Opcode::Multiply, Assoc::Right)
                .parse_infix_op(TokenType::Divide, Precedence::Product, Opcode::Divide, Assoc::Right)
                .parse_infix_op(TokenType::Modulus, Precedence::Product, Opcode::Modulus, Assoc::Right)

                .parse_infix_op(TokenType::DoubleEquals, Precedence::Equality, Opcode::Equal, Assoc::Left)
                .parse_infix_op(TokenType::NotEqual, Precedence::Equality, Opcode::NotEqual, Assoc::Left)
                .parse_infix_op(TokenType::Greater, Precedence::Relational, Opcode::Greater, Assoc::Left)
                .parse_infix_op(TokenType::Less, Precedence::Relational, Opcode::Less, Assoc::Left)
                .parse_infix_op(TokenType::GreaterOrEqual, Precedence::Relational, Opcode::GreaterOrEqual, Assoc::Left)
                .parse_infix_op(TokenType::LessOrEqual, Precedence::Relational, Opcode::LessOrEqual, Assoc::Left);

            match parsed {
                ParsedInfixOp::Complete { opcode, prec } => {
                    self.next();
                    self.parse_value()?;
                    self.parse_infix(prec)?;
                    self.func.bytecode.push(opcode.into());
                }
                ParsedInfixOp::Incomplete { .. } => break,
            }
        }
        Ok(())
    }
    fn parse_expr(&mut self) -> Result<(), MissingInput> {
        self.parse_value()?;
        self.parse_infix(Precedence::Top)?;
        Ok(())
    }
    fn parse_assign_op(&mut self, name: &'src str, opcode: Opcode) -> Result<(), MissingInput> {
        self.next();
        self.parse_expr()?;
        let var = self.resolve_var(name);
        self.func.push_var(var);
        self.func.bytecode.push(opcode.into());
        self.func.pop_var(var);
        Ok(())
    }
    fn parse_if(&mut self) -> Result<(), MissingInput> {
        self.next();
        self.parse_expr()?;
        let cond = self.func.push_jump_if_not();
        self.parse_block()?;
        if self.eat(TokenType::Else) {
            let exit = self.func.push_jump();
            let else_target = self.func.create_jump_target();
            self.func.connect_jump(cond, &else_target);
            if self.token.ty == TokenType::If {
                self.parse_if()?;
            } else {
                self.parse_block()?;
            }
            let end = self.func.create_jump_target();
            self.func.connect_jump(exit, &end);
        } else {
            let end = self.func.create_jump_target();
            self.func.connect_jump(cond, &end);
        }
        Ok(())
    }
    fn parse_stmt(&mut self) -> Result<(), MissingInput> {
        match self.token.ty {
            TokenType::While => {
                self.next();
                let start = self.func.create_jump_target();
                self.parse_expr()?;
                let cond = self.func.push_jump_if_not();
                self.parse_block()?;
                let repeat = self.func.push_jump();
                let exit = self.func.create_jump_target();
                self.func.connect_jump(repeat, &start);
                self.func.connect_jump(cond, &exit);
            }
            TokenType::If => self.parse_if()?,
            TokenType::Var => {
                self.next();
                let name = self.token.source;
                if !self.eat(TokenType::Ident) {
                    self.invalid_token()?;
                }
                self.func.scope.push(name);
                if !self.eat(TokenType::Equals) {
                    self.invalid_token()?;
                }
                let name = self.token.source;
                if self.eat(TokenType::Ident) {
                    if self.token.ty == TokenType::OpenBrace {
                        self.parse_call(name)?;
                    } else {
                        let var = self.resolve_var(name);
                        self.func.push_var(var);
                        self.parse_infix(Precedence::Top)?;
                    }
                } else {
                    self.parse_expr()?;
                }
            }
            TokenType::Print => {
                self.next();
                self.parse_expr()?;
                self.func.bytecode.push(Opcode::PopPrint.into());
            }
            TokenType::Return => {
                self.next();
                self.parse_expr()?;
                self.func.bytecode.extend([Opcode::PopStore.into(), 0]);
            }
            TokenType::Ident => {
                let name = self.token.source;
                self.next();
                match self.token.ty {
                    TokenType::OpenBrace => {
                        self.parse_call(name)?;
                        self.func.bytecode.extend([Opcode::Drop.into(), 1])
                    }
                    TokenType::Equals => {
                        self.next();
                        self.parse_expr()?;
                        let var = self.resolve_var(name);
                        self.func.pop_var(var);
                    }
                    TokenType::PlusEquals => self.parse_assign_op(name, Opcode::Add.into())?,
                    TokenType::MinusEquals => self.parse_assign_op(name, Opcode::Subtract.into())?,
                    TokenType::MultiplyEquals => self.parse_assign_op(name, Opcode::Multiply.into())?,
                    TokenType::DivideEquals => self.parse_assign_op(name, Opcode::Divide.into())?,
                    TokenType::ModulusEquals => self.parse_assign_op(name, Opcode::Modulus.into())?,
                    _ => self.invalid_token()?,
                }
            }
            _ => self.invalid_token()?,
        }
        Ok(())
    }
    fn parse_block(&mut self) -> Result<(), MissingInput> {
        let start_len = self.func.scope.len();
        if !self.eat(TokenType::OpenCurlyBrace) {
            self.invalid_token()?;
        }
        while !self.eat(TokenType::CloseCurlyBrace) {
            self.parse_stmt()?;
        }
        let n = self.func.scope.len() as u8 - start_len as u8;
        if n > 0 {
            self.func.bytecode.extend([Opcode::Drop.into(), n]);
        }
        self.func.scope.truncate(start_len);
        Ok(())
    }
    pub fn parse(source: &'src str) -> Result<Vec<CompletedFunc>, MissingInput> {
        let mut lexer = Lexer::new(source);
        let token = lexer.next_token();
        let mut parser = Parser {
            token,
            lexer,
            func: Func { bytecode: vec![], scope: vec![], param_count: 0, closure_scope: vec![] },
            func_stack: vec![],
            finished_funcs: vec![],
        };
        while !parser.lexer.is_end() {
            parser.parse_stmt()?;
        }
        parser.func.bytecode.push(Opcode::Finish.into());
        parser.finished_funcs.push(CompletedFunc::new(parser.func));
        Ok(parser.finished_funcs)
    }
}