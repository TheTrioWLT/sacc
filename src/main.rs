use sacc::{
    diagnostic::{session::Session, Handler, HandlerFlags, SourceManager},
    lexer::lex,
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

    match session.load_file(&path) {
        Ok(root_src) => {
            if let Ok(tokens) = lex(&session, root_src) {
                for token in tokens.iter() {
                    println!("{:?}", token);
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
