use sacc::{
    diagnostic::{session::Session, Handler, HandlerFlags, SourceManager},
    lexer::lex,
    preprocessor::phase2::phase2,
};
use std::{path::Path, process::exit, rc::Rc};

fn main() {
    let handler_flags = HandlerFlags {
        colored_output: true,
        emit_warnings: true,
        quiet: false,
    };

    let source_manager = Rc::new(SourceManager::new());

    let handler = Handler::with_text_emitter(handler_flags, source_manager.clone());

    let session = Session::new(source_manager, handler);

    let path = Path::new("test.c");

    match session.load_file(path) {
        Ok(root_src) => {
            // Lex tokens from our main source
            if let Ok(tokens) = lex(&session, root_src) {
                // Run phase 2 of translation, which removes comments and backslashes and newlines
                if let Ok(tokens) = phase2(tokens, &session) {
                    for token in tokens.iter() {
                        println!("{:?}", token);
                    }
                }
            }
        }
        Err(e) => {
            session
                .struct_error(format!("couldn't read {:?}: {}", path.as_os_str(), e))
                .emit();

            // Tell the OS we didn't like that
            exit(1);
        }
    };
}
