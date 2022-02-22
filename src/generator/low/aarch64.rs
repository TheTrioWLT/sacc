//! Codegen for aarch64 (ARMV8)
//!
//! This module converts high::Instruction to an aarch64 assembly file. An aarch64 assembler is
//! then invoked to generate an object file

use crate::generator::high::{CompilationUnit, USize64};

pub fn do_codegen(unit: CompilationUnit<'_, '_, USize64>) {

    //TODO what is the return?
}
