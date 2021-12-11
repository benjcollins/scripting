use crate::{lexer::Lexer, opcode::Opcode, token::{Token, TokenType}, func::{Func, FuncBuilder}};

pub struct Parser<'src> {
    lexer: Lexer<'src>,
    token: Token<'src>,
    func_count: u32,
    funcs: Vec<Func<'src>>,
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
    fn parse_call(&mut self, func: &mut FuncBuilder<'src, '_>, name: &'src str) -> Result<(), MissingInput> {
        self.next();
        func.bytecode.push(Opcode::PushNone.into());
        let mut arg_count = 0;
        if !self.eat(TokenType::CloseBrace) {
            loop {
                self.parse_expr(func)?;
                arg_count += 1;
                if !self.eat(TokenType::Comma) {
                    break
                }
            }
            if !self.eat(TokenType::CloseBrace) {
                self.invalid_token()?;
            }
        }
        let var = func.resolve_var(name).unwrap();
        func.push_var(var);
        func.bytecode.extend([Opcode::Call.into(), arg_count]);
        Ok(())
    }
    fn parse_value(&mut self, func: &mut FuncBuilder<'src, '_>) -> Result<(), MissingInput> {
        match self.token.ty {
            TokenType::Ident => {
                let name = self.token.source;
                self.next();
                if self.token.ty == TokenType::OpenBrace {
                    self.parse_call(func, name)?;
                } else {
                    let var = func.resolve_var(name).unwrap();
                    func.push_var(var);
                }
            }
            TokenType::Int => {
                let c: i64 = self.token.source.parse().unwrap();
                self.next();
                func.bytecode.push(Opcode::PushInt.into());
                func.bytecode.extend(c.to_be_bytes());
            }
            TokenType::True => {
                self.next();
                func.bytecode.push(Opcode::PushTrue.into());
            }
            TokenType::False => {
                self.next();
                func.bytecode.push(Opcode::PushFalse.into());
            }
            TokenType::None => {
                self.next();
                func.bytecode.push(Opcode::PushNone.into());
            }
            TokenType::OpenBrace => {
                self.next();
                self.parse_expr(func)?;
                if !self.eat(TokenType::CloseBrace) {
                    self.invalid_token()?;
                }
            }
            TokenType::OpenSquareBrace => {
                self.next();
                let mut length: u32 = 0;
                if !self.eat(TokenType::CloseSquareBrace) {
                    loop {
                        self.parse_expr(func)?;
                        length += 1;
                        if !self.eat(TokenType::Comma) {
                            break
                        }
                    }
                    if !self.eat(TokenType::CloseSquareBrace) {
                        self.invalid_token()?;
                    }
                }
                func.bytecode.push(Opcode::PushList.into());
                func.bytecode.extend(length.to_be_bytes());
            }
            TokenType::Func => {
                self.next();
                func.bytecode.push(Opcode::PushFunc.into());
                self.func_count += 1;
                func.bytecode.extend((self.func_count as u32).to_be_bytes());

                let mut child_func = func.new_child();

                if !self.eat(TokenType::OpenBrace) {
                    self.invalid_token()?;
                }
                if !self.eat(TokenType::CloseBrace) {
                    loop {
                        let name = self.token.source;
                        if !self.eat(TokenType::Ident) {
                            self.invalid_token()?;
                        }
                        child_func.define_param(name);
                        if !self.eat(TokenType::Comma) {
                            break
                        }
                    }
                    if !self.eat(TokenType::CloseBrace) {
                        self.invalid_token()?;
                    }
                }

                if self.token.ty == TokenType::OpenCurlyBrace {
                    self.parse_block(&mut child_func)?;
                } else {
                    self.parse_expr(&mut child_func)?;
                    child_func.bytecode.extend([Opcode::PopStore.into(), 0]);
                }
                child_func.bytecode.push(Opcode::Return.into());
                self.funcs.push(child_func.build());
            }
            _ => self.invalid_token()?,
        };
        Ok(())
    }
    fn parse_infix(&mut self, func: &mut FuncBuilder<'src, '_>, prec: Precedence) -> Result<(), MissingInput> {
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
                    self.parse_value(func)?;
                    self.parse_infix(func, prec)?;
                    func.bytecode.push(opcode.into());
                }
                ParsedInfixOp::Incomplete { .. } => break,
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
        func.bytecode.push(opcode.into());
        func.pop_var(var);
        Ok(())
    }
    fn parse_if(&mut self, func: &mut FuncBuilder<'src, '_>) -> Result<(), MissingInput> {
        self.next();
        self.parse_expr(func)?;
        let cond = func.push_jump_if_not();
        self.parse_block(func)?;
        if self.eat(TokenType::Else) {
            let exit = func.push_jump();
            let else_target = func.create_jump_target();
            func.connect_jump(cond, &else_target);
            if self.token.ty == TokenType::If {
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
        match self.token.ty {
            TokenType::While => {
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
            TokenType::If => self.parse_if(func)?,
            TokenType::Var => {
                self.next();
                let name = self.token.source;
                if !self.eat(TokenType::Ident) {
                    self.invalid_token()?;
                }
                func.define_var(name);
                if !self.eat(TokenType::Equals) {
                    self.invalid_token()?;
                }
                let name = self.token.source;
                if self.eat(TokenType::Ident) {
                    if self.token.ty == TokenType::OpenBrace {
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
            TokenType::Print => {
                self.next();
                self.parse_expr(func)?;
                func.bytecode.push(Opcode::PopPrint.into());
            }
            TokenType::Return => {
                self.next();
                self.parse_expr(func)?;
                func.bytecode.extend([Opcode::PopStore.into(), 0]);
            }
            TokenType::Ident => {
                let name = self.token.source;
                self.next();
                match self.token.ty {
                    TokenType::OpenBrace => {
                        self.parse_call(func, name)?;
                        func.bytecode.extend([Opcode::Drop.into(), 1])
                    }
                    TokenType::Equals => {
                        self.next();
                        self.parse_expr(func)?;
                        let var = func.resolve_var(name).unwrap();
                        func.pop_var(var);
                    }
                    TokenType::PlusEquals => self.parse_assign_op(func, name, Opcode::Add.into())?,
                    TokenType::MinusEquals => self.parse_assign_op(func, name, Opcode::Subtract.into())?,
                    TokenType::MultiplyEquals => self.parse_assign_op(func, name, Opcode::Multiply.into())?,
                    TokenType::DivideEquals => self.parse_assign_op(func, name, Opcode::Divide.into())?,
                    TokenType::ModulusEquals => self.parse_assign_op(func, name, Opcode::Modulus.into())?,
                    _ => self.invalid_token()?,
                }
            }
            _ => self.invalid_token()?,
        }
        Ok(())
    }
    fn parse_block(&mut self, func: &mut FuncBuilder<'src, '_>) -> Result<(), MissingInput> {
        let start_stack_size = func.stack_size();
        if !self.eat(TokenType::OpenCurlyBrace) {
            self.invalid_token()?;
        }
        while !self.eat(TokenType::CloseCurlyBrace) {
            self.parse_stmt(func)?;
        }
        let n = func.stack_size() - start_stack_size;
        if n > 0 {
            func.free_vars(n);
        }
        Ok(())
    }
    pub fn parse(source: &'src str) -> Result<Vec<Func>, MissingInput> {
        let mut lexer = Lexer::new(source);
        let token = lexer.next_token();
        let mut func = FuncBuilder::new();
        let mut parser = Parser {
            token,
            lexer,
            funcs: vec![],
            func_count: 1,
        };
        while parser.token.ty != TokenType::End {
            parser.parse_stmt(&mut func)?;
        }
        func.bytecode.push(Opcode::Finish.into());
        parser.funcs.push(func.build());
        Ok(parser.funcs)
    }
}