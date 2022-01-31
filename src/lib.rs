mod diagnostic;
mod lexer;
mod generator;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
        crate::generator::high::a();
    }
}
