#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Symbol(u32);

#[derive(Debug, Clone)]
pub struct Symbols {
    symbols: Vec<String>,
}

pub const RETURN: Symbol = Symbol(0);

impl Symbol {
    pub fn id(&self) -> u32 {
        self.0
    }
    pub fn from_index(index: u32) -> Symbol {
        Symbol(index)
    }
}

impl Symbols {
    pub fn new() -> Symbols {
        Symbols { symbols: vec!["return".to_string()] }
    }
    pub fn add(&mut self, name: &str) -> Symbol {
        match self.symbols.iter().position(|symbol| *symbol == name) {
            Some(id) => Symbol(id as u32),
            None => {
                let id = self.symbols.len();
                self.symbols.push(name.to_string());
                Symbol(id as u32)
            }
        }
    }
    pub fn get_name(&self, Symbol(id): Symbol) -> &str {
        &self.symbols[id as usize]
    }
}