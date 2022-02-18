use logos::Logos;

use crate::diagnostic::Span;

/// An Enum that represents a token as provided by Logos, which will later be converted into the
/// regular TokenKind after preprocessing
#[derive(Debug, PartialEq, Eq, Clone, Copy, Logos)]
pub enum PTokenKind {
    // Technically the following code is "required" by the C99 spec, but with the limitations of Logos,
    // we cannot lex any input when it is context-dependent, like the HeaderName token. Instead we will
    // parse < followed by anything, followed by another >

    // /// The path to a header file within the C standard library, which is only recognized as such
    // /// by the use of '<' and '>', because other include paths will be recognized as string literals
    // #[regex(r"(?:#include)\s<[^>]+>")]
    // HeaderName,
    //
    //
    /// A valid C identifier
    #[regex(r"[_a-zA-Z][_a-zA-Z0-9]*")]
    Identifier,

    /// A number as recognized by the C preprocessor, which does not have to be a valid parsable
    /// number
    #[regex(r"([0-9]|\.[0-9])([0-9_a-zA-Z\-\+\.])*")]
    Number,

    /// A C character constant
    ///
    /// NOTE: We allow more than one character. This makes our job easier in the lexer and
    /// allows us to deal with hexadecimal and octal escape sequences later
    #[regex("L?\'(?s:[^\'\\\\]|\\\\.)+\'")]
    CharacterConstant,

    /// A string literal
    ///
    /// NOTE: We don't check for allowed escape sequences here, we will do that later so that
    /// we can provide better error messages
    #[regex("L?\"(?s:[^\"\\\\]|\\\\.)*\"")]
    LiteralString,

    /// Left parenthesis
    #[token("(")]
    ParenLeft,

    /// Right parenthesis
    #[token(")")]
    ParenRight,

    /// A comma
    #[token(",")]
    Comma,

    /// A "punctuator" is any operator or symbol, including '{', etc. In order to make
    /// preprocessing slightly easier, (, ), and ',' are their own tokens, but otherwise they would be
    /// considered punctuators as well
    #[regex(r"\[|\]|\{|\}|\.|->|\+\+|\-\-|&|\*|\+|\-|~|!|/|%|<<|>>|<|>|<=|>=|==|!=|\^|\||&&|\|\||\?|:|;|\.\.\.|=|\*=|/=|%=|\+=|\-=|<<=|>>=|&=|\^=|\|=|-|#|##|<:|:>|<%|%>|%:|%:%:")]
    Punctuator,

    /// A cross-platform newline
    #[regex("\r\n|\r|\n")]
    Newline,

    /// A single-line comment
    #[regex(r"//[^\n]*")]
    CommentSingle,

    /// A multi-line comment
    #[regex(r"/\*[^*]*\*+(?:[^/*][^*]*\*+)*/")]
    CommentMulti,

    /// Any non-newline whitespace, which we can't skip for the single reason that: preprocessor
    /// operations
    #[regex("[ \t]+")]
    Whitespace,

    /// A generic error where the lexer encounters something it cannot recognize
    #[error]
    ErrorGeneric,
}

/// A token that has been produced by the lexer from C source code, which is fed into the
/// preprocessor
#[derive(Debug, Clone, Copy)]
pub struct PToken {
    /// The kind of token
    pub kind: PTokenKind,

    /// A index into the SourceManager to the source that this token belongs to, this could be a file or something like
    /// a macro expansion
    pub source: usize,

    /// The start index (by characters) into the source string
    pub start: usize,

    /// The end index (by characters) into the source string
    pub end: usize,
}

impl From<PToken> for Span {
    fn from(token: PToken) -> Self {
        Self {
            start: token.start,
            end: token.end,
            source: token.source,
        }
    }
}
