use core::fmt;
use core::cmp::Ordering;
use std::fmt::Display;
use std::{collections::HashMap, fmt::Debug};
use std::mem::size_of;
use std::convert::TryInto;

use crate::{heap::{Heap, HeapPtr}, opcode::Opcode, list::List, func::{Func, ClosureValue}};

#[derive(Debug, Clone, Copy)]
pub enum Value<'func, 'src> {
    Int(i64),
    Float(f64),
    Bool(bool),
    Closure(HeapPtr<Closure<'func, 'src>>),
    RustValue(HeapPtr<dyn RustValue + 'func>),
    None,
}

pub trait RustValue where Self: Debug + Display {}

#[derive(Debug, Clone)]
pub struct Closure<'func, 'src> {
    func: &'func Func<'src>,
    closure_values: Vec<HeapPtr<ClosureValueRef<'func, 'src>>>,
}

#[derive(Debug, Clone, Copy)]
enum ClosureValueRef<'func, 'src> {
    Stack(usize),
    Heap(HeapPtr<Value<'func, 'src>>),
}

pub struct VirtualMachine<'func, 'src> {
    funcs: &'func [Func<'src>],
    call: Call<'func, 'src>,
    stack: Vec<Value<'func, 'src>>,
    call_stack: Vec<Call<'func, 'src>>,
    heap: Heap,
    finished: bool,
    closure_ref_map: HashMap<usize, Vec<HeapPtr<ClosureValueRef<'func, 'src>>>>,
}

#[derive(Debug, Clone, Copy)]
struct Call<'func, 'src> {
    pc: usize,
    frame: usize,
    closure: HeapPtr<Closure<'func, 'src>>,
}

impl<'func, 'src> PartialEq for Value<'func, 'src> {
    fn eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::None, Value::None) => true,
            _ => false,
        }
    }
}

impl<'func, 'src> Closure<'func, 'src> {
    fn new(
        func: &'func Func<'src>,
        closure: Option<&Closure<'func, 'src>>,
        frame: usize, heap: &mut Heap,
        closure_ref_map: &mut HashMap<usize, Vec<HeapPtr<ClosureValueRef<'func, 'src>>>>
    ) -> Closure<'func, 'src> {
        let closure_values = func.closure_scope.iter().map(|var| match var {
            ClosureValue::Outer(index) => {
                closure.unwrap().closure_values[*index as usize]
            }
            ClosureValue::Stack(rel_index) => {
                let index = frame + *rel_index as usize;
                let closure_ref = heap.alloc(ClosureValueRef::Stack(index));
                closure_ref_map.entry(index).or_insert(vec![]).push(closure_ref);
                closure_ref
            }
        }).collect();
        Closure { func, closure_values }
    }
}

impl<'func, 'src> VirtualMachine<'func, 'src> {
    fn arithmetic_op(&mut self, int: fn(i64, i64) -> i64, float: fn(f64, f64) -> f64) {
        let c = match (self.stack.pop().unwrap(), self.stack.pop().unwrap()) {
            (Value::Int(a), Value::Int(b)) => Value::Int(int(b, a)),
            (Value::Int(a), Value::Float(b)) => Value::Float(float(b, a as f64)),
            (Value::Float(a), Value::Int(b)) => Value::Float(float(b as f64, a)),
            (Value::Float(a), Value::Float(b)) => Value::Float(float(b, a)),
            (a, b) => panic!("invalid operands {} and {}", a, b),
        };
        self.stack.push(c);
    }
    fn comparison_op(&mut self, f: fn(Ordering) -> bool) {
        let ord = match (self.stack.pop().unwrap(), self.stack.pop().unwrap()) {
            (Value::Int(b), Value::Int(a)) => a.cmp(&b),
            (Value::Int(b), Value::Float(a)) => a.partial_cmp(&(b as f64)).unwrap(),
            (Value::Float(b), Value::Int(a)) => (a as f64).partial_cmp(&b).unwrap(),
            (Value::Float(b), Value::Float(a)) => a.partial_cmp(&b).unwrap(),
            _ => panic!(),
        };
        self.stack.push(Value::Bool(f(ord)))
    }
    fn take_bytes(&mut self, n: usize) -> &[u8] {
        let bytes = &self.call.closure.func.bytecode[self.call.pc..self.call.pc + n];
        self.call.pc += n;
        bytes
    }
    fn drop(&mut self) {
        let value = self.stack.pop().unwrap();
        match self.closure_ref_map.remove(&self.stack.len()) {
            Some(ref_list) => {
                if !ref_list.is_empty() {
                    let heap_value = self.heap.alloc(value);
                    for mut closure_ref in ref_list {
                        *closure_ref = ClosureValueRef::Heap(heap_value)
                    }
                }
            }
            _ => (),
        }
    }
    fn step(&mut self) {
        let opcode = self.take_bytes(1)[0].try_into().unwrap();
        match opcode {
            Opcode::Add => self.arithmetic_op(|a, b| a + b, |a, b| a + b),
            Opcode::Subtract => self.arithmetic_op(|a, b| a - b, |a, b| a - b),
            Opcode::Multiply => self.arithmetic_op(|a, b| a * b, |a, b| a * b),
            Opcode::Divide => self.arithmetic_op(|a, b| a / b, |a, b| a / b),
            Opcode::Modulus => self.arithmetic_op(|a, b| a % b, |a, b| a % b),

            Opcode::Equal => {
                let val = self.stack.pop().unwrap() == self.stack.pop().unwrap();
                self.stack.push(Value::Bool(val))
            }
            Opcode::NotEqual => {
                let val = self.stack.pop().unwrap() != self.stack.pop().unwrap();
                self.stack.push(Value::Bool(val))
            }

            Opcode::Less => self.comparison_op(|ord| ord.is_lt()),
            Opcode::Greater => self.comparison_op(|ord| ord.is_gt()),
            Opcode::LessOrEqual => self.comparison_op(|ord| ord.is_le()),
            Opcode::GreaterOrEqual => self.comparison_op(|ord| ord.is_ge()),

            Opcode::PushInt => {
                let bytes = self.take_bytes(size_of::<i64>()).try_into().unwrap();
                self.stack.push(Value::Int(i64::from_be_bytes(bytes)));
            }
            Opcode::PushTrue => self.stack.push(Value::Bool(true)),
            Opcode::PushFalse => self.stack.push(Value::Bool(false)),
            Opcode::PushNone => self.stack.push(Value::None),
            Opcode::PushLoad => {
                let index = self.take_bytes(1)[0] as usize;
                self.stack.push(self.stack[self.call.frame + index])
            }
            Opcode::PushClosureLoad => {
                let index = self.take_bytes(1)[0] as usize;
                self.stack.push(match *self.call.closure.closure_values[index] {
                    ClosureValueRef::Stack(index) => self.stack[index],
                    ClosureValueRef::Heap(ptr) => *ptr,
                });
            }
            Opcode::PushFunc => {
                let func_id = u32::from_be_bytes(self.take_bytes(size_of::<u32>()).try_into().unwrap());
                let closure = Closure::new(
                    &self.funcs[self.funcs.len() - func_id as usize],
                    Some(&self.call.closure),
                    self.call.frame,
                    &mut self.heap,
                    &mut self.closure_ref_map
                );
                self.stack.push(Value::Closure(self.heap.alloc(closure)))
            }
            Opcode::PushList => {
                let length = u32::from_be_bytes(self.take_bytes(size_of::<u32>()).try_into().unwrap()) as usize;
                let list = List::new(&mut self.heap, length, &mut self.stack);
                self.stack.push(Value::RustValue(self.heap.alloc(list)))
            }
            Opcode::PopStore => {
                let index = self.take_bytes(1)[0];
                self.stack[self.call.frame + index as usize] = self.stack.pop().unwrap()
            }
            Opcode::PopClosureStore => {
                let index = self.take_bytes(1)[0] as usize;
                let val = self.stack.pop().unwrap();
                match *self.call.closure.closure_values[index] {
                    ClosureValueRef::Stack(index) => self.stack[index] = val,
                    ClosureValueRef::Heap(mut ptr) => *ptr = val,
                }
            }
            Opcode::PopPrint => println!("{}", self.stack.pop().unwrap()),
            Opcode::Jump => self.call.pc = u32::from_be_bytes(self.take_bytes(size_of::<u32>()).try_into().unwrap()) as usize,
            Opcode::JumpIfNot => {
                let pc = u32::from_be_bytes(self.take_bytes(size_of::<u32>()).try_into().unwrap());
                match self.stack.pop().unwrap() {
                    Value::Bool(b) => if !b {
                        self.call.pc = pc as usize;
                    }
                    val => panic!("{}", val)
                }
            }
            Opcode::Drop => {
                let n = self.take_bytes(1)[0] as usize;
                for _ in 0..n {
                    self.drop()
                }
            }
            Opcode::Call => match self.stack.pop().unwrap() {
                Value::Closure(closure) => {
                    let arg_count = self.take_bytes(1)[0];
                    if arg_count != closure.func.param_count {
                        panic!()
                    }
                    self.call_stack.push(self.call);
                    self.call = Call {
                        pc: 0,
                        frame: self.stack.len() - arg_count as usize - 1,
                        closure,
                    };
                }
                _ => panic!(),
            }
            Opcode::Return => {
                for _ in 0..self.call.closure.func.param_count {
                    self.drop()
                }
                self.call = self.call_stack.pop().unwrap()
            }
            Opcode::Finish => self.finished = true,
        }
        // println!("{}", self.stack.iter().map(|value| format!("{}", value)).collect::<Vec<String>>().join(", "))
    }
    pub fn run(funcs: &[Func], entry_func: &Func) {
        let mut heap = Heap::new();

        let mut closure_ref_map = HashMap::new();
        let closure = Closure::new(entry_func, None, 0, &mut heap, &mut closure_ref_map);

        let mut vm = VirtualMachine {
            funcs,
            call: Call {
                frame: 0,
                closure: heap.alloc(closure),
                pc: 0,
            },
            stack: vec![],
            call_stack: vec![],
            closure_ref_map,
            finished: false,
            heap,
        };

        while !vm.finished {
            vm.step()
        }
    }
}

impl<'func, 'src> fmt::Display for Value<'func, 'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(int) => write!(f, "{}", int),
            Value::Float(float) => write!(f, "{}", float),
            Value::Bool(bool) => write!(f, "{}", bool),
            Value::None => write!(f, "none"),
            Value::Closure(closure) => {
                write!(f, "fn({})", closure.func.scope[1..closure.func.param_count as usize + 1].join(", "))
            }
            Value::RustValue(value) => write!(f, "{}", &**value),
        }
    }
}