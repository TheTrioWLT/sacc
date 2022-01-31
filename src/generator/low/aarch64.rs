//! Codegen for aarch64 (ARMV8)
//!
//! This module converts high::Instruction to an aarch64 assembly file. An aarch64 assembler is
//! then invoked to generate an object file

use crate::generator::high;

pub fn do_codegen(instructions: Vec<high::Instruction<high::USize64>>) {

    //TODO what is the return?
}
