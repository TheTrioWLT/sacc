mod token;
pub use token::{PToken, PTokenKind};

use logos::Logos;

use crate::diagnostic::Source;

/// Runs the Lexer that takes the input source string and produces a Vec<PToken> for later preprocessing
pub fn lex<'a, 'b>(source: &'a str, input_file: &'b Source) -> Vec<PToken<'b, 'a>> {
    let mut tokens = Vec::new();

    let mut lexer = PTokenKind::lexer(source);

    let mut index = 0;

    while let Some(kind) = lexer.next() {
        // Gets the slice of the source code that the current token is from
        let slice = lexer.slice();

        // Convert the raw token into a PToken using the extra data
        let token = PToken {
            kind,
            source: input_file,
            text: slice,
        };

        // FIXME: If this token is an error, for now we will panic
        match token.kind {
            PTokenKind::ErrorGeneric => {
                todo!("Generic lexing error: {:?} at {}", token, index);
            }
            _ => {
                index += token.text.len();
            }
        }

        tokens.push(token);
    }

    tokens
}

#[cfg(test)]
mod tests {
    use crate::diagnostic::Source;

    use super::{PToken, PTokenKind};

    /// FIXME: Just for testing purposes for now so that we can write tests and not have to carry this around every
    /// time
    static DEFAULT_SRC: Source = Source;

    /// Lexes tokens from a provided &str
    fn lex_from_str(source: &str) -> Vec<PToken> {
        super::lex(source, &DEFAULT_SRC)
    }

    /// Checks if the tokens in the first Vec match the kinds provided by the second, skips any
    /// whitespace tokens in the input
    fn check_matches(input: Vec<PToken>, reference: Vec<(PTokenKind, &'static str)>) {
        // Remove whitespace tokens for sanity
        let input: Vec<_> = input
            .into_iter()
            .filter(|i| i.kind != PTokenKind::Whitespace)
            .collect();

        println!("{:#?}", input);

        // Check if they are both the same size
        assert_eq!(input.len(), reference.len());

        // Check each element
        for (token, (kind, text)) in input.iter().zip(reference) {
            assert_eq!(token.kind, kind);
            assert_eq!(token.text, text);
        }
    }

    // Lexes a header name (eg. <stdint.h>)
    #[test]
    fn lex_header_name() {
        let input = lex_from_str("<stdint.h>");

        // Sadly this is the best we can do for now
        let reference = vec![
            (PTokenKind::Punctuator, "<"),
            (PTokenKind::Identifier, "stdint"),
            (PTokenKind::Punctuator, "."),
            (PTokenKind::Identifier, "h"),
            (PTokenKind::Punctuator, ">"),
        ];

        check_matches(input, reference);
    }

    // Lexes various identifiers (eg. __foo__, f020202, etc.)
    #[test]
    fn lex_identifiers() {
        let input = lex_from_str("__foo__ f020202 aWdawnaDa");

        let reference = vec![
            (PTokenKind::Identifier, "__foo__"),
            (PTokenKind::Identifier, "f020202"),
            (PTokenKind::Identifier, "aWdawnaDa"),
        ];

        check_matches(input, reference);
    }

    // Lexes various "numbers" according to the preprocessor, both valid and invalid
    #[test]
    fn lex_numbers() {
        let input = lex_from_str("02 230002 0x2f 0b0_0011 .23f 3.14e+ 3.14e+34 3p3 3.3.4.3.ep+-.3");

        let reference = vec![
            (PTokenKind::Number, "02"),
            (PTokenKind::Number, "230002"),
            (PTokenKind::Number, "0x2f"),
            (PTokenKind::Number, "0b0_0011"),
            (PTokenKind::Number, ".23f"),
            (PTokenKind::Number, "3.14e+"),
            (PTokenKind::Number, "3.14e+34"),
            (PTokenKind::Number, "3p3"),
            (PTokenKind::Number, "3.3.4.3.ep+-.3"),
        ];

        check_matches(input, reference);
    }

    // Lexes various character constants, even those that are invalid such as ones that contain
    // more than one character
    #[test]
    fn lex_characters() {
        let input = lex_from_str("'y' '0' '\\'' '\\0' 'february'");

        let reference = vec![
            (PTokenKind::CharacterConstant, "'y'"),
            (PTokenKind::CharacterConstant, "'0'"),
            (PTokenKind::CharacterConstant, "'\\''"),
            (PTokenKind::CharacterConstant, "'\\0'"),
            (PTokenKind::CharacterConstant, "'february'"),
        ];

        check_matches(input, reference);
    }

    // Lexes string literals
    #[test]
    fn lex_string_literals() {
        let input = lex_from_str(r#" "february" "  has spaces " "021031d s \" " "why? \n" "s" "#);

        let reference = vec![
            (PTokenKind::LiteralString, r#""february""#),
            (PTokenKind::LiteralString, r#""  has spaces ""#),
            (PTokenKind::LiteralString, r#""021031d s \" ""#),
            (PTokenKind::LiteralString, r#""why? \n""#),
            (PTokenKind::LiteralString, r#""s""#),
        ];

        check_matches(input, reference);
    }

    // Lexes all of the standard punctuators
    #[test]
    fn lex_punctuators() {
        let input = lex_from_str(
            r#"( ) , [ ] { } . -> ++ -- & * + - ~ ! / % << >> < > <= >= == != ^ | && || ? : ; ... = *= /= %= += -= <<= >>= &= ^= |= # ## <: :> <% %> %: %:%:"#,
        );

        let reference = vec![
            (PTokenKind::ParenLeft, "("),
            (PTokenKind::ParenRight, ")"),
            (PTokenKind::Comma, ","),
            (PTokenKind::Punctuator, "["),
            (PTokenKind::Punctuator, "]"),
            (PTokenKind::Punctuator, "{"),
            (PTokenKind::Punctuator, "}"),
            (PTokenKind::Punctuator, "."),
            (PTokenKind::Punctuator, "->"),
            (PTokenKind::Punctuator, "++"),
            (PTokenKind::Punctuator, "--"),
            (PTokenKind::Punctuator, "&"),
            (PTokenKind::Punctuator, "*"),
            (PTokenKind::Punctuator, "+"),
            (PTokenKind::Punctuator, "-"),
            (PTokenKind::Punctuator, "~"),
            (PTokenKind::Punctuator, "!"),
            (PTokenKind::Punctuator, "/"),
            (PTokenKind::Punctuator, "%"),
            (PTokenKind::Punctuator, "<<"),
            (PTokenKind::Punctuator, ">>"),
            (PTokenKind::Punctuator, "<"),
            (PTokenKind::Punctuator, ">"),
            (PTokenKind::Punctuator, "<="),
            (PTokenKind::Punctuator, ">="),
            (PTokenKind::Punctuator, "=="),
            (PTokenKind::Punctuator, "!="),
            (PTokenKind::Punctuator, "^"),
            (PTokenKind::Punctuator, "|"),
            (PTokenKind::Punctuator, "&&"),
            (PTokenKind::Punctuator, "||"),
            (PTokenKind::Punctuator, "?"),
            (PTokenKind::Punctuator, ":"),
            (PTokenKind::Punctuator, ";"),
            (PTokenKind::Punctuator, "..."),
            (PTokenKind::Punctuator, "="),
            (PTokenKind::Punctuator, "*="),
            (PTokenKind::Punctuator, "/="),
            (PTokenKind::Punctuator, "%="),
            (PTokenKind::Punctuator, "+="),
            (PTokenKind::Punctuator, "-="),
            (PTokenKind::Punctuator, "<<="),
            (PTokenKind::Punctuator, ">>="),
            (PTokenKind::Punctuator, "&="),
            (PTokenKind::Punctuator, "^="),
            (PTokenKind::Punctuator, "|="),
            (PTokenKind::Punctuator, "#"),
            (PTokenKind::Punctuator, "##"),
            (PTokenKind::Punctuator, "<:"),
            (PTokenKind::Punctuator, ":>"),
            (PTokenKind::Punctuator, "<%"),
            (PTokenKind::Punctuator, "%>"),
            (PTokenKind::Punctuator, "%:"),
            (PTokenKind::Punctuator, "%:%:"),
        ];

        check_matches(input, reference);
    }

    // FIXME: Once the final diagnostic system is implemented, this will not panic anymore. This is
    // currently a bad solution, should_panic is bad
    #[test]
    #[should_panic]
    fn lexer_generic_error() {
        let source = "$";

        let input_file = Source;

        let _ = super::lex(source, &input_file);
    }
}
