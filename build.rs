use std::path::PathBuf;

use serde_json::Value;

fn main() {
    // Add link search directory for Oodle
    println!("cargo:rustc-link-search=./oodle");

    // Generate IPC opcodes
    {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/opcodes.json");

        println!("cargo::rerun-if-changed={}", d.to_str().unwrap());

        let mut output_str = "use binrw::binrw;\n".to_string();

        let opcodes_buffer = std::fs::read_to_string(d).unwrap();
        let json: Value = serde_json::from_str(&opcodes_buffer).unwrap();
        for element in json.as_object().unwrap() {
            let key = element.0;
            let opcodes = element.1.as_array().unwrap();

            if !opcodes.is_empty() {
                // beginning
                output_str.push_str("#[binrw]\n");
                output_str.push_str("#[brw(repr = u16)]\n");
                output_str.push_str("#[derive(Clone, PartialEq, Debug)]\n");
                output_str.push_str(&format!("pub enum {key} {{\n"));

                for opcode in opcodes {
                    let opcode = opcode.as_object().unwrap();
                    let name = opcode.get("name").unwrap().as_str().unwrap();
                    let opcode = opcode.get("opcode").unwrap().as_number().unwrap();

                    output_str.push_str(&format!("{name} = {opcode},\n"));
                }

                // end
                output_str.push_str("}\n\n");

                // sizes
                output_str.push_str(&format!("impl {key} {{\n"));
                output_str.push_str("/// Returns the expected size of the data segment of this IPC opcode, _without_ any headers.\n");
                output_str.push_str("pub fn calc_size(&self) -> u32 {\n");
                output_str.push_str("match self {\n");

                for opcode in opcodes {
                    let opcode = opcode.as_object().unwrap();
                    let name = opcode.get("name").unwrap().as_str().unwrap();
                    let size = opcode.get("size").unwrap().as_number().unwrap();

                    output_str.push_str(&format!("{key}::{name} => {size},\n"));
                }

                output_str.push_str("}\n\n");
                output_str.push_str("}\n\n");
                output_str.push_str("}\n\n");
            }
        }

        std::fs::write("src/opcodes.rs", output_str).expect("Failed to write opcodes file!");
    }
}
