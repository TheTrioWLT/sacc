pub mod command_line;
pub fn compiler_config(args: self::command_line::CompilerConfig) {
    println!("{:?}", args);
}
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
