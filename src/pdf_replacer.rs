use std::error::Error;
use std::collections::{BTreeMap, HashMap};
use lopdf::{Document, content::Content, Object};
use crate::cli::ReplaceMap;

use self::unicode_tools::UnicodeMapper;

mod unicode_tools;

pub fn replace_text(doc: &mut Document, page_number: u32, r_map: &Vec<ReplaceMap>) -> Result<(), Box<dyn Error>> {
    let page_unicode_maps = unicode_tools::extract_page_cmaps(doc, page_number)?;
    let mapper = UnicodeMapper::new(page_unicode_maps);

    let page = page_number.saturating_sub(1) as usize;
    let page_id = doc
        .page_iter()
        .nth(page)
        .ok_or(lopdf::Error::PageNumberNotFound(page_number))?;

    let fonts = doc.get_page_fonts(page_id);
    
    let encodings = fonts
        .into_iter()
        .map(|(name, font)| (name, font.get_font_encoding().to_owned()))
        .collect::<BTreeMap<Vec<u8>, String>>();

    let content_data = doc.get_page_content(page_id)?;
    let mut content = Content::decode(&content_data)?;
    let mut current_font: Option<&[u8]> = None;
    let mut current_encoding: Option<&str> = None;

    for operation in &mut content.operations {
        match operation.operator.as_ref() {
            "Tf" => {
                current_font = operation
                    .operands
                    .get(0)
                    .ok_or_else(|| lopdf::Error::Syntax("missing font operand".to_string()))?
                    .as_name().ok();
                current_encoding = encodings.get(current_font.unwrap_or_default()).map(std::string::String::as_str);
            }
            "Tj" => {
                if !valid_encoding(current_encoding) {continue;}

                let operands_flatmap = operation.operands.iter_mut().flat_map(Object::as_str_mut);
                for bytes in operands_flatmap {
                    let mut  modified: bool = false;
                    let mut decoded_text = mapper.decode(&current_font.unwrap_or_default().to_vec(), bytes);
                    println!("{decoded_text}");

                    for record in r_map {
                        if decoded_text.contains(&record.key) {
                            decoded_text = decoded_text.replace(&record.key, &record.value);
                            modified = true;
                        }
                    }

                    if modified {
                        let encoded_bytes = mapper.encode(&current_font.unwrap_or_default().to_vec(), &decoded_text);
                        *bytes = encoded_bytes;
                    }
                }
            }
            "TJ" => {
                if !valid_encoding(current_encoding) {continue;}

                let mut modified: bool = false;
                let mut object_text = String::new();
                collect_text(&mut object_text, &mapper, &current_font.unwrap_or_default().to_vec(), &operation.operands);
                println!("{object_text}");
                for record in r_map {
                    if object_text.contains(&record.key) {
                        object_text = object_text.replace(&record.key, &record.value);
                        modified = true;
                    }
                }

                if modified {
                    let encoded_bytes = Document::encode_text(current_encoding, &object_text);
                    let object_string = Object::String(encoded_bytes, lopdf::StringFormat::Literal);
                    operation.operands = vec![object_string];
                    operation.operator = "Tj".into();
                }
            }
            _ => {
                // println!("OP {:?}", operation);
            }
        }
    }
    let modified_content = content.encode()?;
    let result = doc.change_page_content(page_id, modified_content);
    Ok(result?)
}

fn valid_encoding (encoding: Option<&str>) -> bool {
    match encoding {
        Some(encoding) => {
            match encoding {
                "StandardEncoding" | 
                "MacRomanEncoding" | 
                "MacExpertEncoding" | 
                "WinAnsiEncoding" | 
                "UniGB-UCS2-H" | 
                "UniGB−UTF16−H" |
                "Identity-H" => true,
                _ => false,
            }
        },
        None => false,
    }
}

fn collect_text(text: &mut String, mapper: &UnicodeMapper, font: &Vec<u8>, operands: &[Object]) {
    for operand in operands.iter() {
        match *operand {
            Object::String(ref bytes, _) => {
                // let decoded_text = Document::decode_text(encoding, bytes);
                let decoded_text = mapper.decode(&font, bytes);
                text.push_str(&decoded_text);
            }
            Object::Array(ref arr) => {
                collect_text(text, mapper, font, arr);
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