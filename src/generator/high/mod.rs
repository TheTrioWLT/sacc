//! High level assembly code generation
//!
//! Takes in an AST and produces high level assembly tokens.
//! High level assembly tokens are cross platform and loosely based on ARM and x86 instructions.
//! This lets us write the optimization algorithms once, and apply them to all the backends.
//! Similar to LLVM IR, there are an infinite number of immediate registers and they are all
//! namespaced by the function they reside in. Parameters are passed in the first 1..=N registers.
//! Values are returned via the Return instruction, and any register / value can be returned.
//!

use std::num::NonZeroU16;

use crate::diagnostic::Source;

/// A high level unnamed register
// Use use `NonZeroU16` and give up one value so that the niche optimization can help us.
// Register numbers are arbitrary anyway, so just start at 1
#[derive(Clone, Debug)]
pub struct Register(NonZeroU16);

/// The size of an integer, either 1, 2, 4, or 8 bytes
#[derive(Clone, Debug)]
pub enum IntegerSize {
    B8,
    B16,
    B32,
    B64,
}

/// The possible sizes of a floating point value
#[derive(Clone, Debug)]
pub enum FloatingSize {
    F32,
    F64,
}

/// A 32 bit value
#[derive(Clone, Debug)]
pub struct USize32(u32);

/// A 64 bit value
#[derive(Clone, Debug)]
pub struct USize64(u64);

/// A complete primitive value
#[derive(Clone, Debug)]
pub enum PrimitiveValue {
    Signed(IntegerSize),
    Unsigned(IntegerSize),
    Floating(FloatingSize),
    Pointer,
}

/// A value's location
// TODO: How to convey volatile?
#[derive(Clone, Debug)]
pub enum StorageLocation<USize> {
    /// The value is stored in the register
    Reg(Register),

    /// The value can be found in memory by dereferencing the address in `register`
    DerefReg(Register),

    /// The value can be found by dereferencing a fixed address
    // TODO: We probably dont know the address at this step
    DerefAddr(USize),
}

#[derive(Clone, Debug)]
pub enum JumpCondition {
    Zero,
    NonZero,
}

/// The high level instructions, including their operands and destination
///
/// The math operators only operate on operands of the same type, and similar to x86, operands can
/// be found in registers, at a fixed address, or by dereferencing a pointer in a register
#[derive(Clone, Debug)]
pub enum Instruction<USize> {
    /// Moves a value from one place to another. This is somewhat analogous x86's MOV.
    /// Register to Register, Mem to Mem, Mem to Register, and Register to Mem are all contained
    /// here
    Move {
        src: StorageLocation<USize>,
        dst: StorageLocation<USize>,
        value: PrimitiveValue,
    },

    /// dst = a + b
    Add {
        a: StorageLocation<USize>,
        b: StorageLocation<USize>,
        dst: StorageLocation<USize>,
        value: PrimitiveValue,
    },

    /// dst = a - b
    Subtract {
        a: StorageLocation<USize>,
        b: StorageLocation<USize>,
        dst: StorageLocation<USize>,
        value: PrimitiveValue,
    },

    /// dst = a * b
    Multiply {
        a: StorageLocation<USize>,
        b: StorageLocation<USize>,
        dst: StorageLocation<USize>,
        value: PrimitiveValue,
    },

    /// dst = a / b
    Divide {
        a: StorageLocation<USize>,
        b: StorageLocation<USize>,
        dst: StorageLocation<USize>,
        value: PrimitiveValue,
    },

    /// Calls a function, storing the return value in `return_value`
    Call {
        /// The function we wish to call
        function: FunctionRef,
        return_value: Option<StorageLocation<USize>>,
    },

    /// Returns the specified value, or `None` for void
    // TODO: How will we return structs?
    Return {
        value: StorageLocation<USize>,
    },

    /// Unconditional jump to instruction offset inside the current function
    Jump {
        offset: isize,
    },

    /// Conditional jump to instruction offset inside the current function if value is non zero
    ConditionalJump {
        /// The relative offset from this instruction to jump to
        /// Offset 0 is this instruction, 1 is the next instruction, -10 is 10 instructions before, etc.
        offset: isize,
        // Abstract the flags register away by having the user specify what (most likely a register)
        // value they want to compare with zero. Usually the value of this register will be set by
        // the previous instruction so we don't need to emit an extra instruction. TODO: Maybe add flags?
        value: StorageLocation<USize>,
        condition: JumpCondition,
    },
}

/// Represents a reference to a function
/// This is simply a index into a function inside a `CompilationUnit`
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FunctionRef(usize);

/// Represents a single high level assembled function
#[derive(Clone, Debug)]
pub struct Function<'name, USize> {
    pub name: &'name str,
    pub instructions: Vec<Instruction<USize>>,
}

/// Represents a partially assembled compilation unit with multiple functions
#[derive(Clone, Debug)]
pub struct CompilationUnit<'name, 'source, USize> {
    pub functions: Vec<Function<'name, USize>>,
    //TODO: globals: Vec<???>,
    
    pub source: &'source Source,
}

impl<'name, 'source, USize> CompilationUnit<'name, 'source, USize> {

    /// Returns a reference to the desired function
    fn get_function(&self, function: FunctionRef) -> &Function<'name, USize> {
        &self.functions[function.0]
    }
}
