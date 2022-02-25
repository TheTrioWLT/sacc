use crate::{
    diagnostic::session::Session,
    lexer::{PToken, PTokenKind},
};

/// Phase 1 according to the C specification is replacing trigraph sequences. Because of the nature
/// of preprocessing tokens, and a distaste of looping through every character before it gets to
/// the lexer, that phase will be postponed as it correctly can be. Therefore phase 2 will come
/// first.
///
/// According to the C specification, phase 2 consists of:
///
/// Each instance of a backslash character ( \) immediately followed by a new-line
/// character is deleted, splicing physical source lines to form logical source lines.
/// Only the last backslash on any physical source line shall be eligible for being part
/// of such a splice. A source file that is not empty shall end in a new-line character,
/// which shall not be immediately preceded by a backslash character before any such
/// splicing takes place.
///
/// Therefore this function removes all newlines following a backslash. Because comments also
/// have no effect on the code generated from C, they are also stripped here.
///
pub fn phase2(tokens: Vec<PToken>, session: &Session) -> Result<Vec<PToken>, ()> {
    let mut new_tokens = Vec::with_capacity(tokens.capacity());

    let mut backslash: Option<PToken> = None;
    let mut has_error = false;

    for token in tokens {
        if backslash.is_some() {
            if token.kind == PTokenKind::Newline {
                backslash = None;
            } else if token.kind == PTokenKind::Whitespace {
                session
                    .struct_span_warn(token.into(), "whitespace before newline after `\\`")
                    .emit();
            } else {
                // At this point we don't have to worry about other files being included in the
                // token stream
                let s = session.span_to_string(&token.into()).unwrap();

                session
                    .struct_error(format!("found unexpected token `{}`", s))
                    .span_label(token.into(), "expected newline after `\\`, found this")
                    .emit();

                // We can continue to try, just in case they make the same mistake again?
                has_error = true;
                backslash = None;
            }
        } else {
            if token.kind != PTokenKind::CommentSingle {
                if token.kind == PTokenKind::Backslash {
                    backslash = Some(token);
                } else {
                    new_tokens.push(token);
                }
            }
        }
    }

    if let Some(backslash) = backslash {
        session
            .struct_error("unexpected end of file")
            .span_label(backslash.into(), "after backslash")
            .emit();
        has_error = true;
    }

    if has_error {
        Err(())
    } else {
        Ok(new_tokens)
    }
}
