use std::{error::Error, io::{self, Read}, fs::File};
use cli::{Options, ReplaceMap};
use lopdf::Document;
use structopt::StructOpt;

mod cli;
mod pdf_replacer;

fn main() -> Result<(), Box<dyn Error>>{
    let options = Options::from_args();

    // Buffer to save the json
    let mut buffer = String::new();

    // Read input json to buffer
    match options.json {
        Some(json_path) => {
            let mut json_file = File::open(json_path)?;

            json_file.read_to_string(&mut buffer)?;
        },
        None => {
            // Open stdin
            let mut stdin = io::stdin();
            // Read stdin until EOF
            stdin.read_to_string(&mut buffer)?;
        },
    }

    let r_map: Vec<ReplaceMap> = serde_json::from_str(&buffer)?;

    let mut document = Document::load(options.input)?;

    for (page, _object_id) in document.get_pages() {
        pdf_replacer::replace_text(&mut document, page, &r_map)?;
        // if page >= 2 { break; }
    }

    match options.output {
        Some(output_path) => {
            document.save(output_path)?;
        },
        None => {
            let mut stdout = io::stdout().lock();
            document.save_to(&mut stdout)?;
        },
    }

    Ok(())
}