use clap::Parser;

///Structure that hold the different types of
///flags or arguments.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct CompilerConfig {
    #[clap(short = 'I', name = "directory")]
    ///Specifies external directiories to include files at compile time.
    pub include: String,

    #[clap(short)]
    ///Compiles and assembles, does not link
    pub compile_assemble: bool,

    #[clap(short = 'S', hide_env = true)]
    ///Compiles only
    pub only_compile: bool,

    #[clap(short, name = "file name")]
    ///Specify output file name
    pub output_file: String,
}
