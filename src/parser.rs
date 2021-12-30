use crate::{lexer::Lexer, opcode::Opcode, token::{Token, TokenKind}, func::{Func, FuncBuilder}};

pub struct Parser<'src> {
    lexer: Lexer<'src>,
    token: Token<'src>,
    func_count: u32,
    funcs: Vec<Func<'src>>,
    props: Vec<&'src str>,
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
pub struct MissingInput {}

impl<'src> Parser<'src> {
    fn next(&mut self) {
        self.token = self.lexer.next_token();
    }
    fn eat(&mut self, kind: TokenKind) -> bool {
        if self.token.kind == kind {
            self.next();
            true
        } else {
            false
        }
    }
    fn invalid_token(&mut self) -> Result<(), MissingInput> {
        match self.token.kind {
            TokenKind::End => Err(MissingInput {}),
            token => panic!("{:?}", token),
        }
    }
    fn eat_ident(&mut self) -> Result<&'src str, MissingInput> {
        match self.token.kind {
            TokenKind::Ident(name) => {
                self.next();
                Ok(name)
            }
            _ => {
                self.invalid_token()?;
                unreachable!();
            }
        }
    }
    fn parse_call(&mut self, func: &mut FuncBuilder<'src, '_>, name: &'src str) -> Result<(), MissingInput> {
        self.next();
        func.push_bytes(&[Opcode::PushNone.into()]);
        let mut arg_count = 0;
        if !self.eat(TokenKind::CloseBrace) {
            loop {
                self.parse_expr(func)?;
                arg_count += 1;
                if !self.eat(TokenKind::Comma) {
                    break
                }
            }
            if !self.eat(TokenKind::CloseBrace) {
                self.invalid_token()?;
            }
        }
        let var = func.resolve_var(name).unwrap();
        func.push_var(var);
        func.push_bytes(&[Opcode::Call.into(), arg_count]);
        Ok(())
    }
    fn parse_value(&mut self, func: &mut FuncBuilder<'src, '_>) -> Result<(), MissingInput> {
        match self.token.kind {
            TokenKind::Ident(name) => {
                self.next();
                match self.token.kind {
                    TokenKind::OpenBrace => {
                        self.parse_call(func, name)?;
                    }
                    _ => {
                        let var = func.resolve_var(name).unwrap();
                        func.push_var(var);
                    }
                }
            }
            TokenKind::Int(val) => {
                self.next();
                func.push_bytes(&[Opcode::PushInt.into()]);
                func.push_bytes(&val.to_be_bytes());
            }
            TokenKind::True => {
                self.next();
                func.push_bytes(&[Opcode::PushTrue.into()]);
            }
            TokenKind::False => {
                self.next();
                func.push_bytes(&[Opcode::PushFalse.into()]);
            }
            TokenKind::None => {
                self.next();
                func.push_bytes(&[Opcode::PushNone.into()]);
            }
            TokenKind::OpenBrace => {
                self.next();
                self.parse_expr(func)?;
                if !self.eat(TokenKind::CloseBrace) {
                    self.invalid_token()?;
                }
            }
            TokenKind::List => {
                self.next();
                if !self.eat(TokenKind::OpenBrace) {
                    self.invalid_token()?;
                }
                let mut length: u32 = 0;
                if !self.eat(TokenKind::CloseBrace) {
                    loop {
                        self.parse_expr(func)?;
                        length += 1;
                        if !self.eat(TokenKind::Comma) {
                            break
                        }
                    }
                    if !self.eat(TokenKind::CloseBrace) {
                        self.invalid_token()?;
                    }
                }
                func.push_bytes(&[Opcode::PushList.into()]);
                func.push_bytes(&length.to_be_bytes());
            }
            TokenKind::Func => {
                self.next();
                func.push_bytes(&[Opcode::PushFunc.into()]);
                self.func_count += 1;
                func.push_bytes(&(self.func_count as u32).to_be_bytes());

                let mut child_func = func.new_child();

                if !self.eat(TokenKind::OpenBrace) {
                    self.invalid_token()?;
                }
                if !self.eat(TokenKind::CloseBrace) {
                    loop {
                        let name = self.eat_ident()?;
                        child_func.define_param(name);
                        if !self.eat(TokenKind::Comma) {
                            break
                        }
                    }
                    if !self.eat(TokenKind::CloseBrace) {
                        self.invalid_token()?;
                    }
                }

                if self.token.kind == TokenKind::OpenCurlyBrace {
                    self.parse_block(&mut child_func)?;
                } else {
                    self.parse_expr(&mut child_func)?;
                    child_func.push_bytes(&[Opcode::PopStore.into(), 0]);
                }
                child_func.push_bytes(&[Opcode::Return.into()]);
                self.funcs.push(child_func.build());
            }
            _ => self.invalid_token()?,
        };
        Ok(())
    }
    fn parse_property(&mut self, func: &mut FuncBuilder<'src, '_>) -> Result<(), MissingInput> {
        self.next();
        let prop = self.eat_ident()?;
        let index = match self.props.iter().copied().enumerate().find(|(_, name)| *name == prop) {
            Some((index, _)) => index,
            None => {
                let index = self.props.len();
                self.props.push(prop);
                index
            }
        };
        func.push_bytes(&[Opcode::PushPropLoad.into(), index as u8]);
        if self.eat(TokenKind::Dot) {
            self.parse_property(func)?;
        }
        Ok(())
    }
    fn parse_infix_op(&mut self, func: &mut FuncBuilder<'src, '_>, prec: Precedence) -> Result<(), MissingInput> {
        self.next();
        self.parse_value(func)?;
        self.parse_infix(func, prec)
    }
    fn parse_infix(&mut self, func: &mut FuncBuilder<'src, '_>, prec: Precedence) -> Result<(), MissingInput> {
        loop {
            match self.token.kind {
                TokenKind::Dot => self.parse_property(func)?,

                TokenKind::Plus if prec < Precedence::Sum => self.parse_infix_op(func, Precedence::Sum)?,
                TokenKind::Minus if prec < Precedence::Sum => self.parse_infix_op(func, Precedence::Sum)?,
                TokenKind::Multiply if prec < Precedence::Product => self.parse_infix_op(func, Precedence::Product)?,
                TokenKind::Divide if prec < Precedence::Product => self.parse_infix_op(func, Precedence::Product)?,
                TokenKind::Modulus if prec < Precedence::Product => self.parse_infix_op(func, Precedence::Product)?,

                TokenKind::DoubleEquals if prec < Precedence::Equality => self.parse_infix_op(func, Precedence::Equality)?,
                TokenKind::NotEqual if prec < Precedence::Equality => self.parse_infix_op(func, Precedence::Equality)?,
                TokenKind::Less if prec < Precedence::Relational => self.parse_infix_op(func, Precedence::Relational)?,
                TokenKind::LessOrEqual if prec < Precedence::Relational => self.parse_infix_op(func, Precedence::Relational)?,
                TokenKind::Greater if prec < Precedence::Relational => self.parse_infix_op(func, Precedence::Relational)?,
                TokenKind::GreaterOrEqual if prec < Precedence::Relational => self.parse_infix_op(func, Precedence::Relational)?,

                _ => break
            }
        }
        Ok(())
    }
    fn parse_expr(&mut self, func: &mut FuncBuilder<'src, '_>) -> Result<(), MissingInput> {
        self.parse_value(func)?;
        self.parse_infix(func, Precedence::Top)?;
        Ok(())
    }
    fn parse_assign_op(&mut self, func: &mut FuncBuilder<'src, '_>, name: &'src str, opcode: Opcode) -> Result<(), MissingInput> {
        self.next();
        self.parse_expr(func)?;
        let var = func.resolve_var(name).unwrap();
        func.push_var(var);
        func.push_bytes(&[opcode.into()]);
        func.pop_var(var);
        Ok(())
    }
    fn parse_if(&mut self, func: &mut FuncBuilder<'src, '_>) -> Result<(), MissingInput> {
        self.next();
        self.parse_expr(func)?;
        let cond = func.push_jump_if_not();
        self.parse_block(func)?;
        if self.eat(TokenKind::Else) {
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
    fn parse_stmt(&mut self, func: &mut FuncBuilder<'src, '_>) -> Result<(), MissingInput> {
        match self.token.kind {
            TokenKind::While => {
                self.next();
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
                self.next();
                let name = self.eat_ident()?;
                func.define_var(name);
                if !self.eat(TokenKind::Equals) {
                    self.invalid_token()?;
                }
                if let TokenKind::Ident(name) = self.token.kind {
                    self.next();
                    if self.token.kind == TokenKind::OpenBrace {
                        self.parse_call(func, name)?;
                    } else {
                        let var = func.resolve_var(name).unwrap();
                        func.push_var(var);
                        self.parse_infix(func, Precedence::Top)?;
                    }
                } else {
                    self.parse_expr(func)?;
                }
            }
            TokenKind::Print => {
                self.next();
                self.parse_expr(func)?;
                func.push_bytes(&[Opcode::PopPrint.into()]);
            }
            TokenKind::Return => {
                self.next();
                self.parse_expr(func)?;
                func.push_bytes(&[Opcode::PopStore.into(), 0]);
            }
            TokenKind::Ident(name) => {
                self.next();
                match self.token.kind {
                    TokenKind::OpenBrace => {
                        self.parse_call(func, name)?;
                        func.push_bytes(&[Opcode::Drop.into(), 1])
                    }
                    TokenKind::Equals => {
                        self.next();
                        self.parse_expr(func)?;
                        let var = func.resolve_var(name).unwrap();
                        func.pop_var(var);
                    }
                    TokenKind::PlusEquals => self.parse_assign_op(func, name, Opcode::Add.into())?,
                    TokenKind::MinusEquals => self.parse_assign_op(func, name, Opcode::Subtract.into())?,
                    TokenKind::MultiplyEquals => self.parse_assign_op(func, name, Opcode::Multiply.into())?,
                    TokenKind::DivideEquals => self.parse_assign_op(func, name, Opcode::Divide.into())?,
                    TokenKind::ModulusEquals => self.parse_assign_op(func, name, Opcode::Modulus.into())?,
                    _ => self.invalid_token()?,
                }
            }
            _ => self.invalid_token()?,
        }
        Ok(())
    }
    fn parse_block(&mut self, func: &mut FuncBuilder<'src, '_>) -> Result<(), MissingInput> {
        let start_stack_size = func.stack_size();
        if !self.eat(TokenKind::OpenCurlyBrace) {
            self.invalid_token()?;
        }
        while !self.eat(TokenKind::CloseCurlyBrace) {
            self.parse_stmt(func)?;
        }
        let n = func.stack_size() - start_stack_size;
        if n > 0 {
            func.free_vars(n);
        }
        Ok(())
    }
    pub fn parse(source: &'src str) -> Result<(Vec<Func>, Vec<&'src str>), MissingInput> {
        let mut lexer = Lexer::new(source);
        let token = lexer.next_token();
        let mut func = FuncBuilder::new();
        let mut parser = Parser {
            token,
            lexer,
            funcs: vec![],
            func_count: 1,
            props: vec![],
        };
        while parser.token.kind != TokenKind::End {
            parser.parse_stmt(&mut func)?;
        }
        func.push_bytes(&[Opcode::Finish.into()]);
        parser.funcs.push(func.build());
        Ok((parser.funcs, parser.props))
    }
}