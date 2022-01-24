use core::fmt;

use crate::{lexer::Lexer, opcode::Opcode, token::{Token, TokenKind, pos_at_offset}, func::{Func, FuncBuilder}, symbols::{Symbols, Symbol}};

pub struct Parser<'a> {
    source: &'a str,
    path: Option<&'a str>,
    lexer: Lexer<'a>,
    token: Token<'a>,
    program: &'a mut Program,
}

pub struct Program {
    pub funcs: Vec<Func>,
    pub symbols: Symbols,
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
enum Precedence {
    Product,
    Sum,
    Relational,
    Equality,
    Top,
}

#[derive(Debug, Clone, Copy)]
pub enum ParseError<'src> {
    InvalidInput(InvalidInput<'src>),
    EndOfInput,
}

#[derive(Debug, Clone, Copy)]
pub struct InvalidInput<'src> {
    source: &'src str,
    path: Option<&'src str>,
    offset: usize,
}

impl<'src> fmt::Display for InvalidInput<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pos = pos_at_offset(self.source, self.offset);
        match self.path {
            Some(path) => writeln!(f, "syntax error at {}:{}:{}", path, pos.line, pos.column),
            None => writeln!(f, "syntax error at {}:{}", pos.line, pos.column),
        }?;
        writeln!(f, "'{}'", self.source.lines().nth(pos.line as usize - 1).unwrap())?;
        for _ in 0..pos.column {
            write!(f, " ")?;
        }
        write!(f, "^")
    }
}

impl Program {
    pub fn new() -> Program {
        Program { funcs: vec![], symbols: Symbols::new() }
    }
}

impl<'a> Parser<'a> {
    fn next_token(&mut self) {
        self.token = self.lexer.next_token();
    }
    fn eat_token(&mut self, kind: TokenKind<'a>) -> bool {
        if self.token.kind == kind {
            self.next_token();
            true
        } else {
            false
        }
    }
    fn expect_ident(&mut self) -> Result<&'a str, ParseError<'a>> {
        match self.token.kind {
            TokenKind::Ident(name) => {
                self.next_token();
                Ok(name)
            }
            _ => Err(self.parse_error()),
        }
    }
    fn expect_token(&mut self, kind: TokenKind<'a>) -> Result<(), ParseError<'a>> {
        if self.eat_token(kind) {
            Ok(())
        } else {
            Err(self.parse_error())
        }
    }
    fn parse_error(&mut self) -> ParseError<'a> {
        match self.token.kind {
            TokenKind::End => ParseError::EndOfInput,
            _ => ParseError::InvalidInput(InvalidInput {
                path: self.path,
                source: self.source,
                offset: self.token.offset,
            })
        }
    }
    fn parse_call(&mut self, func: &mut FuncBuilder<'a, '_>, symbol: Symbol) -> Result<(), ParseError<'a>> {
        self.next_token();
        func.push_bytes(&[Opcode::PushNone.into()]);
        let mut arg_count = 0;
        if !self.eat_token(TokenKind::CloseBrace) {
            loop {
                self.parse_expr(func)?;
                arg_count += 1;
                if !self.eat_token(TokenKind::Comma) {
                    break
                }
            }
            self.expect_token(TokenKind::CloseBrace)?;
        }
        let var = func.resolve_var(symbol).unwrap();
        func.push_var(var);
        func.push_bytes(&[Opcode::Call.into(), arg_count]);
        Ok(())
    }
    fn parse_value(&mut self, func: &mut FuncBuilder<'a, '_>) -> Result<(), ParseError<'a>> {
        match self.token.kind {
            TokenKind::Ident(name) => {
                self.next_token();
                let symbol = self.program.symbols.add(name);
                match self.token.kind {
                    TokenKind::OpenBrace => {
                        self.parse_call(func, symbol)?;
                    }
                    _ => {
                        let var = func.resolve_var(symbol).unwrap();
                        func.push_var(var);
                    }
                }
            }
            TokenKind::Int(val) => {
                self.next_token();
                func.push_bytes(&[Opcode::PushInt.into()]);
                func.push_bytes(&val.to_be_bytes());
            }
            TokenKind::Float(val) => {
                self.next_token();
                func.push_bytes(&[Opcode::PushFloat.into()]);
                func.push_bytes(&val.to_be_bytes());
            }
            TokenKind::True => {
                self.next_token();
                func.push_bytes(&[Opcode::PushTrue.into()]);
            }
            TokenKind::False => {
                self.next_token();
                func.push_bytes(&[Opcode::PushFalse.into()]);
            }
            TokenKind::None => {
                self.next_token();
                func.push_bytes(&[Opcode::PushNone.into()]);
            }
            TokenKind::OpenBrace => {
                self.next_token();
                self.parse_expr(func)?;
                self.expect_token(TokenKind::CloseBrace)?;
            }
            TokenKind::List => {
                self.next_token();
                self.expect_token(TokenKind::OpenBrace)?;
                let mut length: u32 = 0;
                if !self.eat_token(TokenKind::CloseBrace) {
                    loop {
                        self.parse_expr(func)?;
                        length += 1;
                        if !self.eat_token(TokenKind::Comma) {
                            break
                        }
                    }
                    self.expect_token(TokenKind::CloseBrace)?;
                }
                func.push_bytes(&[Opcode::PushList.into()]);
                func.push_bytes(&length.to_be_bytes());
            }
            TokenKind::Func => {
                self.next_token();
                let func_index = self.program.funcs.len();
                self.program.funcs.push(Func::default());
                func.push_bytes(&[Opcode::PushFunc.into()]);
                func.push_bytes(&(func_index as u32).to_be_bytes());

                let mut child_func = func.new_child();

                self.expect_token(TokenKind::OpenBrace)?;
                if !self.eat_token(TokenKind::CloseBrace) {
                    loop {
                        let name = self.expect_ident()?;
                        let symbol = self.program.symbols.add(name);
                        child_func.define_param(symbol);
                        if !self.eat_token(TokenKind::Comma) {
                            break
                        }
                    }
                    self.expect_token(TokenKind::CloseBrace)?;
                }

                if self.token.kind == TokenKind::OpenCurlyBrace {
                    self.parse_block(&mut child_func)?;
                } else {
                    self.parse_expr(&mut child_func)?;
                    child_func.push_bytes(&[Opcode::PopStore.into(), 0]);
                }
                child_func.push_bytes(&[Opcode::Return.into()]);
                self.program.funcs[func_index] = child_func.build();
            }
            _ => return Err(self.parse_error()),
        };
        Ok(())
    }
    fn parse_property(&mut self, func: &mut FuncBuilder<'a, '_>) -> Result<(), ParseError<'a>> {
        self.next_token();
        let name = self.expect_ident()?;
        let symbol = self.program.symbols.add(name);
        func.push_bytes(&[Opcode::PushPropLoad.into(), symbol.id() as u8]);
        if self.eat_token(TokenKind::Dot) {
            self.parse_property(func)?;
        }
        Ok(())
    }
    fn parse_infix_op(&mut self, func: &mut FuncBuilder<'a, '_>, prec: Precedence, op: Opcode) -> Result<(), ParseError<'a>> {
        self.next_token();
        self.parse_value(func)?;
        self.parse_infix(func, prec)?;
        func.push_bytes(&[op.into()]);
        Ok(())
    }
    fn parse_infix(&mut self, func: &mut FuncBuilder<'a, '_>, prec: Precedence) -> Result<(), ParseError<'a>> {
        loop {
            match self.token.kind {
                TokenKind::Dot => self.parse_property(func)?,

                TokenKind::Plus if prec > Precedence::Sum => self.parse_infix_op(func, Precedence::Sum, Opcode::Add)?,
                TokenKind::Minus if prec > Precedence::Sum => self.parse_infix_op(func, Precedence::Sum, Opcode::Subtract)?,
                TokenKind::Multiply if prec > Precedence::Product => self.parse_infix_op(func, Precedence::Product, Opcode::Multiply)?,
                TokenKind::Divide if prec > Precedence::Product => self.parse_infix_op(func, Precedence::Product, Opcode::Divide)?,
                TokenKind::Modulus if prec > Precedence::Product => self.parse_infix_op(func, Precedence::Product, Opcode::Modulus)?,

                TokenKind::DoubleEquals if prec > Precedence::Equality => self.parse_infix_op(func, Precedence::Equality, Opcode::Equal)?,
                TokenKind::NotEqual if prec > Precedence::Equality => self.parse_infix_op(func, Precedence::Equality, Opcode::NotEqual)?,
                TokenKind::Less if prec > Precedence::Relational => self.parse_infix_op(func, Precedence::Relational, Opcode::Less)?,
                TokenKind::LessOrEqual if prec > Precedence::Relational => self.parse_infix_op(func, Precedence::Relational, Opcode::LessOrEqual)?,
                TokenKind::Greater if prec > Precedence::Relational => self.parse_infix_op(func, Precedence::Relational, Opcode::Greater)?,
                TokenKind::GreaterOrEqual if prec > Precedence::Relational => self.parse_infix_op(func, Precedence::Relational, Opcode::GreaterOrEqual)?,

                _ => break
            }
        }
        Ok(())
    }
    fn parse_expr(&mut self, func: &mut FuncBuilder<'a, '_>) -> Result<(), ParseError<'a>> {
        self.parse_value(func)?;
        self.parse_infix(func, Precedence::Top)?;
        Ok(())
    }
    fn parse_assign_op(&mut self, func: &mut FuncBuilder<'a, '_>, symbol: Symbol, opcode: Opcode) -> Result<(), ParseError<'a>> {
        self.next_token();
        self.parse_expr(func)?;
        let var = func.resolve_var(symbol).unwrap();
        func.push_var(var);
        func.push_bytes(&[opcode.into()]);
        func.pop_var(var);
        Ok(())
    }
    fn parse_if(&mut self, func: &mut FuncBuilder<'a, '_>) -> Result<(), ParseError<'a>> {
        self.next_token();
        self.parse_expr(func)?;
        let cond = func.push_jump_if_not();
        self.parse_block(func)?;
        if self.eat_token(TokenKind::Else) {
            let exit = func.push_jump();
            let else_target = func.create_jump_target();
            func.connect_jump(cond, &else_target);
            if self.token.kind == TokenKind::If {
                self.parse_if(func)?;
            } else {
                self.parse_block(func)?;
            }
            let end = func.create_jump_target();
            func.connect_jump(exit, &end);
        } else {
            let end = func.create_jump_target();
            func.connect_jump(cond, &end);
        }
        Ok(())
    }
    fn parse_stmt(&mut self, func: &mut FuncBuilder<'a, '_>) -> Result<(), ParseError<'a>> {
        match self.token.kind {
            TokenKind::While => {
                self.next_token();
                let start = func.create_jump_target();
                self.parse_expr(func)?;
                let cond = func.push_jump_if_not();
                self.parse_block(func)?;
                let repeat = func.push_jump();
                let exit = func.create_jump_target();
                func.connect_jump(repeat, &start);
                func.connect_jump(cond, &exit);
            }
            TokenKind::If => self.parse_if(func)?,
            TokenKind::Var => {
                self.next_token();
                let name = self.expect_ident()?;
                let symbol = self.program.symbols.add(name);
                func.define_var(symbol);
                self.expect_token(TokenKind::Equals)?;
                if let TokenKind::Ident(name) = self.token.kind {
                    let symbol = self.program.symbols.add(name);
                    self.next_token();
                    if self.token.kind == TokenKind::OpenBrace {
                        self.parse_call(func, symbol)?;
                    } else {
                        let var = func.resolve_var(symbol).unwrap();
                        func.push_var(var);
                        self.parse_infix(func, Precedence::Top)?;
                    }
                } else {
                    self.parse_expr(func)?;
                }
            }
            TokenKind::Print => {
                self.next_token();
                self.parse_expr(func)?;
                func.push_bytes(&[Opcode::PopPrint.into()]);
            }
            TokenKind::Return => {
                self.next_token();
                self.parse_expr(func)?;
                func.push_bytes(&[Opcode::PopStore.into(), 0]);
                func.push_bytes(&[Opcode::Return.into()]);
            }
            TokenKind::Ident(name) => {
                self.next_token();
                let symbol = self.program.symbols.add(name);
                match self.token.kind {
                    TokenKind::OpenBrace => {
                        self.parse_call(func, symbol)?;
                        func.push_bytes(&[Opcode::Drop.into(), 1])
                    }
                    TokenKind::Equals => {
                        self.next_token();
                        self.parse_expr(func)?;
                        let var = func.resolve_var(symbol).unwrap();
                        func.pop_var(var);
                    }
                    TokenKind::PlusEquals => self.parse_assign_op(func, symbol, Opcode::Add.into())?,
                    TokenKind::MinusEquals => self.parse_assign_op(func, symbol, Opcode::Subtract.into())?,
                    TokenKind::MultiplyEquals => self.parse_assign_op(func, symbol, Opcode::Multiply.into())?,
                    TokenKind::DivideEquals => self.parse_assign_op(func, symbol, Opcode::Divide.into())?,
                    TokenKind::ModulusEquals => self.parse_assign_op(func, symbol, Opcode::Modulus.into())?,
                    _ => return Err(self.parse_error()),
                }
            }
            _ => return Err(self.parse_error()),
        }
        Ok(())
    }
    fn parse_block(&mut self, func: &mut FuncBuilder<'a, '_>) -> Result<(), ParseError<'a>> {
        let start_stack_size = func.stack_size();
        self.expect_token(TokenKind::OpenCurlyBrace)?;
        while !self.eat_token(TokenKind::CloseCurlyBrace) {
            self.parse_stmt(func)?;
        }
        let n = func.stack_size() - start_stack_size;
        if n > 0 {
            func.free_vars(n);
        }
        Ok(())
    }
    pub fn parse(source: &'a str, path: Option<&'a str>, program: &'a mut Program, params: Vec<Symbol>) -> Result<Vec<Symbol>, ParseError<'a>> {
        let mut lexer = Lexer::new(source);
        let token = lexer.next_token();
        let func_index = program.funcs.len();
        program.funcs.push(Func::default());
        let mut func = FuncBuilder::new(source, params);
        let mut parser = Parser { path, source, token, lexer, program };
        while parser.token.kind != TokenKind::End {
            parser.parse_stmt(&mut func)?;
        }
        func.push_bytes(&[Opcode::Finish.into()]);
        let last_scope = func.scope.clone();
        parser.program.funcs[func_index] = func.build();
        Ok(last_scope)
    }
}