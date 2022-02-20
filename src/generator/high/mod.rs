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

use crate::diagnostic::SourceIndex;

/// A high level unnamed register
// Use use `NonZeroU16` and give up one value so that the niche optimization can help us.
// Register numbers are arbitrary anyway, so just start at 1
pub struct Register(NonZeroU16);

/// The size of an integer, either 1, 2, 4, or 8 bytes
pub enum IntegerSize {
    B8,
    B16,
    B32,
    B64,
}

/// The possible sizes of a floating point value
pub enum FloatingSize {
    F32,
    F64,
}

/// A 32 bit value
pub struct USize32(u32);

/// A 64 bit value
pub struct USize64(u64);

/// A complete primitive value
pub enum PrimitiveValue {
    Signed(IntegerSize),
    Unsigned(IntegerSize),
    Floating(FloatingSize),
    Pointer,
}

/// A value's location
// TODO: How to convey volatile?
pub enum StorageLocation<USize> {
    /// The value is stored in the register
    Reg(Register),

    /// The value can be found in memory by dereferencing the address in `register`
    DerefReg(Register),

    /// The value can be found by dereferencing a fixed address
    // TODO: We probably dont know the address at this step
    DerefAddr(USize),
}

/// The high level instructions, including their operands and destination
///
/// The math operators only operate on operands of the same type, and similar to x86, operands can
/// be found in registers, at a fixed address, or by dereferencing a pointer in a register
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

    /// Calls a function, storing the return value in `return_value`.
    /// Parameters are passin in registers 1..N
    Call {
        // TODO ??? How do we reference functions at this stage
        return_value: Option<StorageLocation<USize>>,
    },

    /// Returns the specified value, or `None` for void
    // TODO: How will we return structs?
    Return {
        value: Option<StorageLocation<USize>>,
    },
}

/// Represents a reference to a function
/// This is simply a index into a function inside a `CompilationUnit`
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FunctionRef(usize);

/// Represents a single high level assembled function
pub struct Function<'name, USize> {
    name: &'name str,
    instructions: Vec<Instruction<USize>>,
}

/// Represents a partially assembled compilation unit with multiple functions
pub struct CompilationUnit<'name, USize> {
    functions: Vec<Function<'name, USize>>,
    //TODO: globals: Vec<???>,
    source: SourceIndex,
}

impl<'name, USize> CompilationUnit<'name, USize> {
    /// Returns a reference to the desired function
    fn get_function(&self, function: FunctionRef) -> &Function<'name, USize> {
        &self.functions[function.0]
    }
}
