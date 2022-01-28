mod token;
pub use token::{Token, TokenKind};

use logos::Logos;

use crate::diagnostic::Source;

/// Runs the Lexer that takes the input source string and produces a Vec<Token> for later parsing
pub fn lex<'a, 'b>(source: &'a str, input_file: &'b Source) -> Vec<Token<'b, 'a>> {
    let mut tokens = Vec::new();

    let mut lexer = TokenKind::lexer(source);

    while let Some(kind) = lexer.next() {
        // Gets the slice of the source code that the current token is from
        let slice = lexer.slice();

        // Convert the raw token into a Token using the extra data
        let token = Token {
            kind,
            source: input_file,
            text: slice,
        };

        // FIXME: If this token is an error, for now we will panic
        match token.kind {
            TokenKind::ErrorGeneric => {
                todo!("Generic lexing error: {:?}", token);
            }
            _ => {}
        }

        tokens.push(token);
    }

    tokens
}

#[cfg(test)]
mod tests {
    use crate::diagnostic::Source;

    use super::TokenKind;

    #[test]
    fn lexer_success() {
        let source = r#"89 "#;

        let input_file = Source;

        let reference_tokens = vec![
            TokenKind::LiteralInteger,
            TokenKind::Whitespace,
            TokenKind::Newline,
        ];

        let tokens = super::lex(source, &input_file);

        for (token, reference) in tokens.iter().zip(reference_tokens.iter()) {
            assert_eq!(token.kind, *reference);
        }
    }

    // FIXME: Once the final diagnostic system is implemented, this will not panic anymore. This is
    // currently a bad solution, should_panic is bad
    #[test]
    #[should_panic]
    fn lexer_generic_error() {
        let source = r#"$"#;

        let input_file = Source;

        let _ = super::lex(source, &input_file);
    }
}
