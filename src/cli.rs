use std::path::PathBuf;
use structopt::StructOpt;
use serde::Deserialize;

#[derive(Debug, StructOpt)]
#[structopt(name = "pdf_replacer", about = "Replace simple strings in pdf documents.")]
pub struct Options {
    /// JSON Array file path or read from stdin until EOF if not present
    /// [
    ///     {key: "[PLACEHOLDER]", value: "A Value"}, 
    ///     {key: "anything", value: "Other value"}
    /// ]
    #[structopt(short, parse(from_os_str), verbatim_doc_comment)]
    pub json: Option<PathBuf>,

    /// PDF Source path
    #[structopt(short, parse(from_os_str))]
    pub input: PathBuf,

    /// PDF Output file path or outputs to stdout if not present
    #[structopt(short, parse(from_os_str))]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
pub struct ReplaceMap {
    pub key: String,
    pub value: String
}