// In many places within this codebase, we manually emit an error and return Result<_, ()> as there
// is no state that needs to be passed back to the caller
#![allow(clippy::result_unit_err)]
pub mod diagnostic;
pub mod generator;
pub mod lexer;
