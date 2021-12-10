#[derive(Clone, Copy)]
pub union CompactValue {
    int: u64,
    float: f64,
}

const TAG_MASK: u64 = 0x0000000000000003;
const PTR_MASK: u64 = 0x0003fffffffffffc;
const NAN_MASK: u64 = 0x7ffc000000000000;
const BOOL_BIT: u64 = 0x0000000000000004;
const SIGN_BIT: u64 = 0x8000000000000000;
const SIGN_EXT: u64 = 0xffff000000000000;

const NONE_TAG: u64 = 0;
const INT_TAG: u64 = 1;
const BOOL_TAG: u64 = 2;
const PTR_TAG: u64 = 3;

impl CompactValue {
    pub fn decode(&self) -> Value {
        unsafe {
            if self.float.is_nan() {
                match self.int & TAG_MASK {
                    NONE_TAG => Value::None,
                    // PTR_TAG => Value::Ptr(self.int & PTR_MASK),
                    BOOL_TAG => Value::Bool(BOOL_BIT & self.int != 0),
                    INT_TAG => if SIGN_BIT & self.int == 0 {
                        Value::Int((self.int >> 2 & !SIGN_EXT) as i64)
                    } else {
                        Value::Int((self.int >> 2 | SIGN_EXT) as i64)
                    },
                    _ => unreachable!(),
                }
            } else {
                Value::Float(self.float)
            }
        }
    }
    pub fn encode(val: Value) -> CompactValue {
        match val {
            Value::Int(i) => CompactValue { int: NAN_MASK | ((i as u64) << 2) | INT_TAG | if i < 0 { SIGN_BIT } else { 0 } },
            Value::Float(float) => CompactValue { float },
            // Value::Ptr(p) => CompactValue { int: NAN_MASK | p | PTR_TAG },
            Value::Bool(b) => CompactValue { int: NAN_MASK | if b { BOOL_BIT } else { 0 } | BOOL_TAG },
            Value::None => CompactValue { int: NAN_MASK | NONE_TAG },
        }
    }
}