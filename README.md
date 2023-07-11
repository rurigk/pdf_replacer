A simple cli PDF text replacer written in rust using lopdf

### Requeriments
- Rust build tools (Only for building)

### Rust Setup
Just install rust following the official [Install Rust](https://www.rust-lang.org/tools/install) guide

### Building
`cargo build --release`

### Running

##### Arguments
```
-h, --help      Prints help information
-i <input>      PDF Source path
-j <json>       JSON Array file path or read from stdin until EOF if not present
                [
                    {key: "[PLACEHOLDER]", value: "A Value"}, 
                    {key: "anything", value: "Other value"}
                ]
-o <output>     PDF Output file path or outputs to stdout if not present
```

##### Examples
`pdf_replacer -h` for help

`pdf_replacer -i template.pdf` reads pdf from file and reads replace map from stdin until EOF and outputs pdf data to stdout

`pdf_replacer -i template.pdf -j map.json` reads pdf from file and reads replace map from file and outputs pdf data to stdout

`pdf_replacer -i template.pdf -j map.json -o final.pdf` reads pdf from file and reads replace map from file and outputs pdf data to a file