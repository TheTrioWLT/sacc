use crate::diagnostic::Source;
use logos::Logos;

/// An Enum that represents a token as provided by Logos, which will later be converted into the
/// regular Token, but still stores which kind of token it is, hence the name
#[derive(Debug, PartialEq, Eq, Clone, Copy, Logos)]
pub enum TokenKind {
    #[regex("[0-9]+")]
    LiteralInteger,

    #[regex("[ \t]+")]
    Whitespace,

    #[regex("\r\n|\r|\n")]
    Newline,

    #[error]
    ErrorGeneric,
}

/// A token that has been produced by the lexer from C source code
#[derive(Debug, Clone, Copy)]
pub struct Token<'a, 'b> {
    // The kind of token
    pub kind: TokenKind,

    // A reference to the source that this token belongs to, this could be a file or something like
    // a macro expansion
    pub source: &'a Source,

    // The text that this token represents, a section of a source file
    pub text: &'b str,
}
