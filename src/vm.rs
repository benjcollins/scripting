use core::cmp::Ordering;
use std::{collections::HashMap, fmt::Debug};
use std::mem::size_of;
use std::convert::TryInto;

use crate::parser::Program;
use crate::value::{Value, ClosureValueRef, Closure, DispValue};
use crate::{heap::{Heap, HeapPtr}, opcode::Opcode, list::List, func::{Func, ClosureValue}};

pub struct VirtualMachine<'a> {
    pub program: &'a Program,
    call: Call,
    stack: &'a mut Vec<Value>,
    call_stack: Vec<Call>,
    heap: &'a mut Heap,
    finished: bool,
    closure_ref_map: HashMap<usize, Vec<HeapPtr<ClosureValueRef>>>,
}

#[derive(Debug, Clone, Copy)]
struct Call {
    pc: usize,
    frame: usize,
    closure: HeapPtr<Closure>,
}

impl PartialEq for Value {
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

impl Closure {
    fn new(
        func_id: usize,
        closure: Option<&Closure>,
        frame: usize,
        heap: &mut Heap,
        closure_ref_map: &mut HashMap<usize, Vec<HeapPtr<ClosureValueRef>>>,
        funcs: &[Func],
    ) -> Closure {
        let closure_values = funcs[func_id].closure_scope.iter().map(|var| match var {
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
        Closure { func_id, closure_values }
    }
}

impl<'a> VirtualMachine<'a> {
    fn arithmetic_op(&mut self, int: fn(i64, i64) -> i64, float: fn(f64, f64) -> f64) {
        let c = match (self.stack.pop().unwrap(), self.stack.pop().unwrap()) {
            (Value::Int(a), Value::Int(b)) => Value::Int(int(b, a)),
            (Value::Int(a), Value::Float(b)) => Value::Float(float(b, a as f64)),
            (Value::Float(a), Value::Int(b)) => Value::Float(float(b as f64, a)),
            (Value::Float(a), Value::Float(b)) => Value::Float(float(b, a)),
            (a, b) => panic!("invalid operands {} and {}", DispValue::new(a, self.program), DispValue::new(b, self.program)),
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
        let func = &self.program.funcs[self.call.closure.func_id];
        let bytes = &func.bytecode[self.call.pc..self.call.pc + n];
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
            Opcode::PushFloat => {
                let bytes = self.take_bytes(size_of::<f64>()).try_into().unwrap();
                self.stack.push(Value::Float(f64::from_be_bytes(bytes)));
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
            Opcode::PushPropLoad => {
                let index = self.take_bytes(1)[0];
                match self.stack.pop().unwrap() {
                    Value::RustValue(mut value) => {
                        let prop = value.get_property(index, self);
                        self.stack.push(prop);
                    }
                    _ => panic!(),
                }
            }
            Opcode::PushFunc => {
                let func_id = u32::from_be_bytes(self.take_bytes(size_of::<u32>()).try_into().unwrap());
                let closure = Closure::new(
                    func_id as usize,
                    Some(&self.call.closure),
                    self.call.frame,
                    &mut self.heap,
                    &mut self.closure_ref_map,
                    &self.program.funcs,
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
            Opcode::PopPropStore => {
                todo!()
            }
            Opcode::PopPrint => {
                let value = self.stack.pop().unwrap();
                println!("{}", DispValue::new(value, self.program))
            }
            Opcode::Jump => self.call.pc = u32::from_be_bytes(self.take_bytes(size_of::<u32>()).try_into().unwrap()) as usize,
            Opcode::JumpIfNot => {
                let pc = u32::from_be_bytes(self.take_bytes(size_of::<u32>()).try_into().unwrap());
                match self.stack.pop().unwrap() {
                    Value::Bool(b) => if !b {
                        self.call.pc = pc as usize;
                    }
                    value => panic!("{}", DispValue::new(value, self.program))
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
                    if arg_count != self.program.funcs[closure.func_id].param_count {
                        panic!()
                    }
                    self.call_stack.push(self.call);
                    self.call = Call {
                        pc: 0,
                        frame: self.stack.len() - arg_count as usize - 1,
                        closure,
                    };
                }
                value => panic!("{}", DispValue::new(value, self.program)),
            }
            Opcode::Return => {
                for _ in 0..self.program.funcs[self.call.closure.func_id].param_count {
                    self.drop()
                }
                self.call = self.call_stack.pop().unwrap()
            }
            Opcode::Finish => self.finished = true,
        }
    }
    pub fn run(program: &Program, entry_func: usize, stack: &mut Vec<Value>, heap: &mut Heap) {
        let mut closure_ref_map = HashMap::new();
        let closure = Closure::new(entry_func, None, 0, heap, &mut closure_ref_map, &program.funcs);

        let mut vm = VirtualMachine {
            program,
            call: Call {
                frame: 0,
                closure: heap.alloc(closure),
                pc: 0,
            },
            stack,
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