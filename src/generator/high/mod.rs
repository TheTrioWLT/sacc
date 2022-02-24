//! High level assembly code generation
//!
//! Takes in an AST and produces high level assembly tokens.
//! High level assembly tokens are cross platform and loosely based on ARM and x86 instructions.
//! This lets us write the optimization algorithms once, and apply them to all the backends.
//! Similar to LLVM IR, there are an infinite number of immediate registers and they are all
//! namespaced by the function they reside in. Values are returned via the Return instruction.
//! Any register / value can be returned.
//!
//! Only architecture specific optimazitons are applied at the low level, so it is the
//! responsibility of the high level to optimize the cross arch assembly as much as possible
//! because the lower levels generate basically expactly what they are given
//!

use std::{fmt::Binary, num::NonZeroU16};

use crate::diagnostic::SourceIndex;

/// A high level unnamed register
// Use use `NonZeroU16` and give up one value so that the niche optimization can help us.
// Register numbers are arbitrary anyway, so just start at 1
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Register(NonZeroU16);

/// The size of an integer, either 1, 2, 4, or 8 bytes
#[derive(Copy, Clone, Debug)]
pub enum IntegerSize {
    B8,
    B16,
    B32,
    B64,
}

/// The possible sizes of a floating point value
#[derive(Copy, Clone, Debug)]
pub enum FloatingSize {
    F32,
    F64,
}

pub trait USizeBase: Copy + Clone + Eq {}

/// A 32 bit value
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct USize32(u32);

/// A 64 bit value
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct USize64(u64);

impl USizeBase for USize32 {}
impl USizeBase for USize64 {}

/// A complete primitive value
#[derive(Copy, Clone, Debug)]
pub enum PrimitiveValue {
    Signed(IntegerSize),
    Unsigned(IntegerSize),
    Floating(FloatingSize),
    Pointer,
}

/// A value this has a writable location
// TODO: How to convey volatile?
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LValue<USize: USizeBase> {
    /// The value is stored in the register
    Reg(Register),

    /// The value can be found in memory by dereferencing the address in `register`
    DerefReg(Register),

    /// The value can be found by dereferencing a fixed address
    // TODO: We probably dont know the address at this step.
    // Maybe use some kind of `GlobalRef` like how we have FuncitonRef? But then code like this:
    // ```
    // int* addr = (int*) 0x02000000;
    // int a = *addr
    // ```
    // would have to be 2 high level instructions:
    // `{Move(tmp_register, 0x02000000), Move(var_a, DerefReg(tmp_register))]`
    // and without re-optimizing this in the low level generator back into one instruction, we
    // would loose some performanace
    DerefAddr(USize),
}

/// A value with a readable location. Can be an LValue or a literal
#[derive(Copy, Clone, Debug)]
pub enum RValue<USize: USizeBase> {
    Writeable(LValue<USize>),
    Literal(usize), // TODO: How do we store any value that can be read? an enum?
}

#[derive(Clone, Debug)]
pub enum JumpCondition {
    Zero,
    NonZero,
}

#[derive(Copy, Clone, Debug)]
pub struct BinaryOperator<USize: USizeBase> {
    pub a: RValue<USize>,
    pub b: RValue<USize>,
    pub dst: LValue<USize>,
    pub value: PrimitiveValue,
}

/// The high level instructions, including their operands and destination
///
/// The math operators only operate on operands of the same type, and similar to x86, operands can
/// be found in registers, at a fixed address, or by dereferencing a pointer in a register
#[derive(Clone, Debug)]
pub enum Instruction<USize: USizeBase> {
    /// Moves a value from one place to another. This is somewhat analogous x86's MOV.
    /// Register to Register, Mem to Mem, Mem to Register, and Register to Mem are all contained
    /// here
    Move {
        src: RValue<USize>,
        dst: LValue<USize>,
        value: PrimitiveValue,
    },

    /// Loads the nth parameter from arguments into the specified register. This is the only way to
    /// access parameters because at this stage we don't know if this architecture supports passing
    /// parameters in registers or the stack.
    LoadParameter { n: u8, dst: Register },

    // TODO: Maybe use structs to store all inner data here to make matches nicer?
    /// dst = a + b
    Add(BinaryOperator<USize>),

    /// dst = a - b
    Subtract(BinaryOperator<USize>),

    /// dst = a * b
    Multiply(BinaryOperator<USize>),

    /// dst = a / b
    Divide(BinaryOperator<USize>),

    /// Calls a function, storing the return value in `return_value`.
    /// Parameters are passin in registers 1..N
    Call {
        /// The function we wish to call
        function: FunctionRef,
        return_value: Option<LValue<USize>>,
    },

    /// Returns the specified value, or `None` for void
    // TODO: How will we return structs?
    Return { value: RValue<USize> },

    /// Unconditional jump to instruction offset inside the current function
    Jump { offset: isize },

    /// Conditional jump to instruction offset inside the current function if value is non zero
    ConditionalJump {
        /// The relative offset from this instruction to jump to
        /// Offset 0 is this instruction, 1 is the next instruction, -10 is 10 instructions before, etc.
        offset: isize,
        // Abstract the flags register away by having the user specify what (most likely a register)
        // value they want to compare with zero. Usually the value of this register will be set by
        // the previous instruction so we don't need to emit an extra instruction. TODO: Maybe add flags?
        value: LValue<USize>,
        condition: JumpCondition,
    },
}

/// Represents a reference to a function
/// This is simply a index into a function inside a `CompilationUnit`
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FunctionRef(usize);

/// Represents a single high level assembled function
#[derive(Clone, Debug)]
pub struct Function<'name, USize: USizeBase> {
    pub name: &'name str,
    pub instructions: Vec<Instruction<USize>>,
}

/// Represents a partially assembled compilation unit with multiple functions
#[derive(Clone, Debug)]
pub struct CompilationUnit<'name, USize: USizeBase> {
    functions: Vec<Function<'name, USize>>,
    //TODO: globals: Vec<???>,
    source: SourceIndex,
}

/// A helper struct for allocaning unique registers
pub struct RegisterAllocator(u16);

impl<'name, USize: USizeBase> Function<'name, USize> {
    pub fn new(name: &'name str, instrunctions: impl Into<Vec<Instruction<USize>>>) -> Self {
        Self {
            name,
            instructions: instrunctions.into(),
        }
    }

    pub fn compute_ins_offset(&self, index: usize, offset: isize) -> Result<usize, ()> {
        let u: usize = (offset + index as isize)
            .try_into()
            .expect("BUG: internal offset out of range");
        if u >= self.instructions.len() {
            //Out of positive range
            Err(())
        } else {
            Ok(u)
        }
    }
}

impl<'name, USize: USizeBase> CompilationUnit<'name, USize> {
    /// Returns a reference to the desired function
    fn get_function(&self, function: FunctionRef) -> &Function<'name, USize> {
        &self.functions[function.0]
    }
}

impl RegisterAllocator {
    pub fn new() -> Self {
        Self(0)
    }

    /// Allocates the next unique register
    pub fn alloc(&mut self) -> Register {
        self.0 = self
            .0
            .checked_add(1)
            .expect("Register id overflow. Too many registers allocated!");
        Register(NonZeroU16::new(self.0).unwrap())
    }
}

impl Iterator for RegisterAllocator {
    type Item = Register;
    fn next(&mut self) -> Option<Self::Item> {
        Some(self.alloc())
    }
}

impl<USize: USizeBase> BinaryOperator<USize> {
    /// Converts this format `c = a <operation> b` to `a <operation>= b`
    /// This only works for some kinds of self, where dest is the same as a or b, so this function
    /// returns an option when the conversion cannot be made.
    /// This function will also re-order a and b so that the destination is the first return value.
    ///
    /// I.E if self is { a: Lit(5), b: r2, dst: r2}, then the return value will be `(r2, Lit(5))`, because
    /// the result of the binary operation is stored in r2.
    pub fn to_two_args(self) -> Option<(LValue<USize>, RValue<USize>)> {
        if let RValue::Writeable(a) = self.a {
            if self.dst == a {
                return Some((a, self.b));
            }
        }
        if let RValue::Writeable(b) = self.b {
            if self.dst == b {
                return Some((b, self.a));
            }
        }
        None
    }
}
