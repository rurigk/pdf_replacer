use std::{collections::{BTreeMap, HashMap}, error::Error};
use lopdf::{Document, Object};

pub fn extract_page_cmaps(doc: &Document, page_number: u32) -> Result<BTreeMap<Vec<u8>, Font>, Box<dyn Error>> {
    // This is "ToUnicode" dictionary key
    let to_unicode: Vec<u8> = vec![84, 111, 85, 110, 105, 99, 111, 100, 101];

    // Convert page to usize
    let page = page_number.saturating_sub(1) as usize;
    // Get page id
    let page_id = doc
        .page_iter()
        .nth(page)
        .ok_or(lopdf::Error::PageNumberNotFound(page_number))?;

    // Get page fonts
    let fonts = doc.get_page_fonts(page_id);

    let mut unicode_maps: BTreeMap<Vec<u8>, Font> = BTreeMap::new();

    // Iterate over each font
    for (font_id, font_dict) in fonts {
        // Obtain "ToUnicode" object
        let unicode_map = if let Ok(to_unicode_objet) = font_dict.get(&to_unicode) {
            match to_unicode_objet {
                Object::Stream(stream) => {
                    // Get decompressed stream bytes
                    if let Ok(to_unicode_cmap_bytes) = stream.decompressed_content() {
                        if let Ok(to_unicode_map) = adobe_cmap_parser::get_unicode_map(&to_unicode_cmap_bytes) {
                            Some((to_unicode_map.clone(), transform_unicode_map(&to_unicode_map)))
                        } else { None }
                    } else { None }
                },
                Object::Reference(id) => {
                    // Get object from document
                    if let Ok(object_stream) = doc.get_object(*id) {
                        // Convert object to stream
                        if let Ok(stream) = object_stream.as_stream() {
                            // Get decompressed stream bytes
                            if let Ok(to_unicode_cmap_bytes) = stream.decompressed_content() {
                                if let Ok(to_unicode_map) = adobe_cmap_parser::get_unicode_map(&to_unicode_cmap_bytes) {
                                    Some((to_unicode_map.clone(), transform_unicode_map(&to_unicode_map)))
                                } else { None }
                            } else { None }
                        } else { None }
                    } else { None }
                },
                _ => panic!("Unknown ToUnicode Object type")
            }
        } else { None };
        unicode_maps.insert(font_id, Font::new(font_dict.get_font_encoding().to_owned(), unicode_map));
    }

    Ok(unicode_maps)
}

fn transform_unicode_map(cmap: &HashMap<u32, Vec<u8>>) -> HashMap<u32, String> {
    let mut unicode = HashMap::new();
    // "It must use the beginbfchar, endbfchar, beginbfrange, and endbfrange operators to
    // define the mapping from character codes to Unicode character sequences expressed in
    // UTF-16BE encoding."
    for (&k, v) in cmap.iter() {
        let mut be: Vec<u16> = Vec::new();
        let mut i = 0;
        assert!(v.len() % 2 == 0);
        while i < v.len() {
            be.push(((v[i] as u16) << 8) | v[i+1] as u16);
            i += 2;
        }
        match &be[..] {
            [0xd800 ..= 0xdfff] => {
                // this range is not specified as not being encoded
                // we ignore them so we don't an error from from_utt16
                continue;
            }
            _ => {}
        }
        let s = String::from_utf16(&be).unwrap();

        unicode.insert(k, s);
    }
    unicode
}

#[derive(Debug)]
pub struct Font {
    encoding: String,
    unicode_raw_map: Option<HashMap<u32, Vec<u8>>>,
    unicode_map: Option<HashMap<u32, String>>
}

impl Font {
    pub fn new(encoding: String, unicode_map: Option<(HashMap<u32, Vec<u8>>, HashMap<u32, String>)>) -> Self {
        if let Some((unicode_raw_map, unicode_map)) = unicode_map {
            Self { encoding, unicode_map: Some(unicode_map), unicode_raw_map: Some(unicode_raw_map) }
        } else {
            Self { encoding, unicode_map: None, unicode_raw_map: None }
        }
    }
}

#[derive(Debug)]
pub struct UnicodeMapper {
    maps: BTreeMap<Vec<u8>, Font>
}

impl UnicodeMapper {
    pub fn new(maps: BTreeMap<Vec<u8>, Font>) -> Self {
        Self { maps }
    }

    pub fn decode(&self, font: &Vec<u8>, bytes: &[u8]) -> String {
        let font = self.maps.get(font);

        let value = match font {
            Some(font) => {
                match font.encoding.as_str() {
                    "Identity-H" => {
                        let mut text = String::new();
                        for byte in bytes.iter() {
                            let map = font.unicode_map.as_ref().unwrap();
                            if let Some(char) = map.get(&u32::from(*byte)) {
                                text.push_str(&char);
                            }
                        }
                        text
                    }
                    _ => {
                        Document::decode_text(Some(&font.encoding), bytes)
                    }
                }
            },
            None => "?NOFONT?".to_string(),
        };

        value
    }

    pub fn encode(&self, font: &Vec<u8>, text: &str) -> Vec<u8> {
        let font = self.maps.get(font);
        match font {
            Some(font) => {
                match font.encoding.as_str() {
                    "Identity-H" => {
                        vec![]
                    }
                    _ => {
                        Document::encode_text(Some(&font.encoding), text)
                    }
                }
            },
            None => vec![],
        }
    }
}