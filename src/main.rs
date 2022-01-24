use clap::StructOpt;
use sacc::command_line::CompilerConfig;

fn main() {
    let args = CompilerConfig::parse();

    sacc::compiler_config(args);
}
