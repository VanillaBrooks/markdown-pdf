mod data;
mod latex;
mod parse;
mod postprocess;

use argh::FromArgs;
use std::path::PathBuf;

#[derive(FromArgs)]
/// Generate presentations in latex from markdown
struct MarkdownPdfArguments {
    #[argh(positional)]
    /// path/to/markdown.md
    markdown_input: PathBuf,

    #[argh(positional)]
    /// path/to/your/output/dir
    output_directory: PathBuf,
}

fn main() {
    if let Err(e) = wrapper() {
        println!("An Error: {}", e)
    }
}

fn wrapper() -> Result<(), Error> {
    let mut args: MarkdownPdfArguments = argh::from_env();

    let file_name = args
        .markdown_input
        .file_stem()
        .ok_or(Error::BadFileName)?
        .to_str()
        .ok_or(Error::NonUtf8Filename)?;

    args.output_directory.push(format!("{}.tex", file_name));

    let f = std::fs::File::open(args.markdown_input)?;
    let parse_results = parse::parse_markdown(f)?;
    let processed_results = postprocess::postprocess(parse_results);

    let out = std::fs::File::create(args.output_directory)?;

    latex::write_latex(out, processed_results)?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("IoError occured: `{0}`")]
    Io(#[from] std::io::Error),
    #[error("Markdown text was not encoded as UTF-8: `{0}`")]
    Encoding(#[from] std::string::FromUtf8Error),
    #[error("The markdown input was not a valid filename")]
    BadFileName,
    #[error("The markdown file name provided was not UTF-8")]
    NonUtf8Filename,
    #[error("Parsing Error")]
    Nom,
}
