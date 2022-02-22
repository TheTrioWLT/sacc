//! Low level assembly generation.
//! There is a separate mod for each supported backend architutrute.
//! Currently the following are supported:
//! - aarch64
//! - ARMv7 (proposed)
//! - x86_64 (proposed)
//!
//! Register allocation occurs in this step

use super::high::{CompilationUnit, USize64, USize32};

mod aarch64;
mod x86_64;

#[derive(Clone, Debug)]
pub enum Backend<'name> {
    Aarch64(CompilationUnit<'name, USize64>),
    Armv7(CompilationUnit<'name, USize32>),
    X86_64(CompilationUnit<'name, USize64>),
}

pub fn do_codegen<'name, USize>(unit: CompilationUnit<'name, USize>, backend: Backend) /* -> WHAT */
{
    match backend {
        Backend::Aarch64(unit) => aarch64::do_codegen(unit),
        Backend::Armv7(unit) => unimplemented!(),
        Backend::X86_64(unit) => x86_64::do_codegen(unit).unwrap(),
    }
}
