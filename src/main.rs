use std::{error::Error, io::{self, Read}, fs::File, collections::BTreeMap};
use lopdf::{Document, content::Content, Object};
use std::path::PathBuf;
use structopt::StructOpt;
use serde::Deserialize;

#[derive(Debug, StructOpt)]
#[structopt(name = "pdf_replacer", about = "Replace simple strings in pdf documents.")]
struct Options {
    /// JSON Array file path or read from stdin until EOF if not present
    /// [
    ///     {key: "[PLACEHOLDER]", value: "A Value"}, 
    ///     {key: "anything", value: "Other value"}
    /// ]
    #[structopt(short, parse(from_os_str), verbatim_doc_comment)]
    json: Option<PathBuf>,

    /// PDF Source path
    #[structopt(short, parse(from_os_str))]
    input: PathBuf,

    /// PDF Output file path or outputs to stdout if not present
    #[structopt(short, parse(from_os_str))]
    output: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct ReplaceMap {
    key: String,
    value: String
}

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

    let rmap: Vec<ReplaceMap> = serde_json::from_str(&buffer)?;

    let mut document = Document::load(options.input)?;

    for (page, _object_id) in document.get_pages() {
        for record in &rmap {
            replace_text(&mut document, page, &record.key, &record.value)?;
        }   
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

pub fn replace_text(doc: &mut Document, page_number: u32, text: &str, other_text: &str) -> Result<(), Box<dyn Error>> {
    let page = page_number.saturating_sub(1) as usize;
    let page_id = doc
        .page_iter()
        .nth(page)
        .ok_or(lopdf::Error::PageNumberNotFound(page_number))?;
    let encodings = doc
        .get_page_fonts(page_id)
        .into_iter()
        .map(|(name, font)| (name, font.get_font_encoding().to_owned()))
        .collect::<BTreeMap<Vec<u8>, String>>();

    let content_data = doc.get_page_content(page_id)?;
    let mut content = Content::decode(&content_data)?;
    let mut current_encoding = None;
    for operation in &mut content.operations {
        match operation.operator.as_ref() {
            "Tf" => {
                let current_font = operation
                    .operands
                    .get(0)
                    .ok_or_else(|| lopdf::Error::Syntax("missing font operand".to_string()))?
                    .as_name()?;
                current_encoding = encodings.get(current_font).map(std::string::String::as_str);
            }
            "Tj" => {
                let operands_flatmap = operation.operands.iter_mut().flat_map(Object::as_str_mut);
                for bytes in operands_flatmap {
                    let decoded_text = Document::decode_text(current_encoding, bytes);
                    if decoded_text == text {
                        let encoded_bytes = Document::encode_text(current_encoding, other_text);
                        *bytes = encoded_bytes;
                    }
                }
            }
            "TJ" => {
                let mut object_text = String::new();
                collect_text(&mut object_text, current_encoding, &operation.operands);
                if object_text.contains(text) {
                    let new_text: String = object_text.replace(text, other_text);
                    let encoded_bytes = Document::encode_text(current_encoding, &new_text);
                    let object_string = Object::String(encoded_bytes, lopdf::StringFormat::Literal);
                    operation.operands = vec![object_string];
                    operation.operator = "Tj".into();
                }
            }
            _ => {}
        }
    }
    let modified_content = content.encode()?;
    let result = doc.change_page_content(page_id, modified_content);
    Ok(result?)
}

fn collect_text(text: &mut String, encoding: Option<&str>, operands: &[Object]) {
    for operand in operands.iter() {
        match *operand {
            Object::String(ref bytes, _) => {
                let decoded_text = Document::decode_text(encoding, bytes);
                text.push_str(&decoded_text);
            }
            Object::Array(ref arr) => {
                collect_text(text, encoding, arr);
                text.push(' ');
            }
            Object::Integer(i) => {
                if i < -100 {
                    text.push(' ');
                }
            }
            _ => {}
        }
    }
}