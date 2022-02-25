#![allow(clippy::result_unit_err)]
pub mod diagnostic;
pub mod lexer;
pub mod preprocessor;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
