mod token;
use std::rc::Rc;

pub use token::{PToken, PTokenKind};

use logos::Logos;

use crate::diagnostic::{session::Session, SourceFile};

pub type LexResult = Result<Vec<PToken>, ()>;

/// Runs the Lexer that takes the input source string and produces a Vec<PToken> for later preprocessing
pub fn lex(session: &Session, input_file: Rc<SourceFile>) -> LexResult {
    let mut tokens = Vec::new();

    let source = input_file.src.as_ref().unwrap();

    let mut lexer = PTokenKind::lexer(source);

    let mut index = 0;

    // This keeps track of if an error was encountered, because the lexer will keep emitting errors
    // as long as it has them to get the maximum amount of information out, this will determine if
    // we had an error or not after lexing is complete
    let mut had_error = false;

    // This keeps track of if we are in a multi-line comment, which will have to be removed at this
    // stage because it becomes a burden if it must be removed at a later stage, as it does not
    // affect any of the actual code
    let mut multi_comment_start: Option<PToken> = None;

    while let Some(kind) = lexer.next() {
        // Gets the slice of the source code that the current token is from
        let slice = lexer.slice();

        // Convert the raw token into a PToken using the extra data
        let token = PToken {
            kind,
            source: input_file.index,
            start: index,
            end: index + slice.len(),
        };

        if token.kind == PTokenKind::CommentMultiStart {
            if let Some(comment_start) = multi_comment_start {
                session
                    .struct_span_warn(comment_start.into(), "`/*` within block comment")
                    .note("block comments cannot be nested")
                    .emit();
            } else {
                multi_comment_start = Some(token);
            }
        } else if token.kind == PTokenKind::CommentMultiEnd {
            if multi_comment_start.is_some() {
                multi_comment_start = None;
            } else {
                session
                    .struct_error("unexpected token `*/`")
                    .span_label(token.into(), "lone block comment terminator")
                    .emit();

                had_error = true;
            }
        } else if multi_comment_start.is_none() {
            if token.kind == PTokenKind::ErrorGeneric {
                let text = session.span_to_string(&token.into()).unwrap();

                session
                    .struct_error(format!("error lexing token `{}`", text))
                    .span_label(token.into(), "invalid token found")
                    .emit();

                had_error = true;
            }

            tokens.push(token);
        }

        index += slice.len();
    }

    if let Some(comment_start) = multi_comment_start {
        session
            .struct_span_error(
                comment_start.into(),
                "Unterminated block comment begins here",
            )
            .emit();

        had_error = true;
    }

    if !had_error {
        Ok(tokens)
    } else {
        Err(())
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::diagnostic::{session::Session, Handler, SourceFile, SourceManager};

    use super::{PToken, PTokenKind};

    /// Creates a dummy session
    fn dummy_sess(source: &str) -> (Session, Rc<SourceFile>) {
        let source_manager = Rc::new(SourceManager::new());

        let handler = Handler::with_text_emitter(
            crate::diagnostic::HandlerFlags {
                colored_output: false,
                emit_warnings: false,
                quiet: true,
            },
            source_manager.clone(),
        );

        let session = Session::new(source_manager.clone(), handler);

        let source_file = source_manager.create_dummy_file(source);

        (session, source_file)
    }

    /// Checks if the tokens in the first Vec match the kinds provided by the second, skips any
    /// whitespace tokens in the input
    fn check_matches(
        src: Rc<SourceFile>,
        input: Vec<PToken>,
        reference: Vec<(PTokenKind, &'static str)>,
    ) {
        // Remove whitespace tokens for sanity
        let input: Vec<_> = input
            .into_iter()
            .filter(|i| i.kind != PTokenKind::Whitespace)
            .collect();

        println!("{:#?}", input);

        // Check if they are both the same size
        assert_eq!(input.len(), reference.len());

        // Check each element
        for (&token, (kind, text)) in input.iter().zip(reference) {
            assert_eq!(token.kind, kind);
            assert_eq!(src.span_to_string(&token.into()).unwrap(), text);
        }
    }

    // Lexes a header name (eg. <stdint.h>)
    #[test]
    fn lex_header_name() {
        let (sess, src) = dummy_sess("<stdint.h>");

        let input = super::lex(&sess, src.clone()).unwrap();

        // Sadly this is the best we can do for now
        let reference = vec![
            (PTokenKind::Punctuator, "<"),
            (PTokenKind::Identifier, "stdint"),
            (PTokenKind::Punctuator, "."),
            (PTokenKind::Identifier, "h"),
            (PTokenKind::Punctuator, ">"),
        ];

        check_matches(src, input, reference);
    }

    // Lexes various identifiers (eg. __foo__, f020202, etc.)
    #[test]
    fn lex_identifiers() {
        let (sess, src) = dummy_sess("__foo__ f020202 aWdawnaDa");

        let input = super::lex(&sess, src.clone()).unwrap();

        let reference = vec![
            (PTokenKind::Identifier, "__foo__"),
            (PTokenKind::Identifier, "f020202"),
            (PTokenKind::Identifier, "aWdawnaDa"),
        ];

        check_matches(src, input, reference);
    }

    // Lexes various "numbers" according to the preprocessor, both valid and invalid
    #[test]
    fn lex_numbers() {
        let (sess, src) =
            dummy_sess("02 230002 0x2f 0b0_0011 .23f 3.14e+ 3.14e+34 3p3 3.3.4.3.ep+-.3");

        let input = super::lex(&sess, src.clone()).unwrap();

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

        check_matches(src, input, reference);
    }

    // Lexes various character constants, even those that are invalid such as ones that contain
    // more than one character
    #[test]
    fn lex_characters() {
        let (sess, src) = dummy_sess("'y' '0' '\\'' '\\0' 'february'");

        let input = super::lex(&sess, src.clone()).unwrap();

        let reference = vec![
            (PTokenKind::CharacterConstant, "'y'"),
            (PTokenKind::CharacterConstant, "'0'"),
            (PTokenKind::CharacterConstant, "'\\''"),
            (PTokenKind::CharacterConstant, "'\\0'"),
            (PTokenKind::CharacterConstant, "'february'"),
        ];

        check_matches(src, input, reference);
    }

    // Lexes string literals
    #[test]
    fn lex_string_literals() {
        let (sess, src) =
            dummy_sess(r#" "february" "  has spaces " "021031d s \" " "why? \n" "s" "#);

        let input = super::lex(&sess, src.clone()).unwrap();

        let reference = vec![
            (PTokenKind::LiteralString, r#""february""#),
            (PTokenKind::LiteralString, r#""  has spaces ""#),
            (PTokenKind::LiteralString, r#""021031d s \" ""#),
            (PTokenKind::LiteralString, r#""why? \n""#),
            (PTokenKind::LiteralString, r#""s""#),
        ];

        check_matches(src, input, reference);
    }

    // Lexes all of the standard punctuators
    #[test]
    fn lex_punctuators() {
        let (sess, src) = dummy_sess(
            r#"( ) , [ ] { } . -> ++ -- & * + - ~ ! / % << >> < > <= >= == != ^ | && || ? : ; ... = *= /= %= += -= <<= >>= &= ^= |= # ## <: :> <% %> %: %:%: \"#,
        );

        let input = super::lex(&sess, src.clone()).unwrap();

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
            (PTokenKind::Backslash, "\\"),
        ];

        check_matches(src, input, reference);
    }

    #[test]
    fn lex_comments() {
        let (sess, src) = dummy_sess(
            r#"// This is a single line comment
/*
 * This is a multi-line comment
 */"#,
        );

        let input = super::lex(&sess, src.clone()).unwrap();

        // NOTE: Multi-line comments are stripped during lexing, and therefore should not show up
        // here
        let reference = vec![
            (
                PTokenKind::CommentSingle,
                r#"// This is a single line comment"#,
            ),
            (PTokenKind::Newline, "\n"),
        ];

        check_matches(src, input, reference);
    }

    /// This should_panic because GenericError emitting in the context of a test actually causes
    /// the error handling logic to fail, which will be fixed in a newer version. In which case
    /// this test will fail and will be fixed.
    #[test]
    #[should_panic]
    fn lexer_generic_error() {
        let source = "$";

        let (sess, src) = dummy_sess(source);

        if let Ok(_) = super::lex(&sess, src) {
            panic!("Input should have generated GenericError");
        }
    }
}
