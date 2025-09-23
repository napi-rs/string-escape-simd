use std::fs;

use json_escape_simd::{escape, escape_generic};

fn main() {
    for fixture in get_rxjs_sources() {
        let encoded = escape(&fixture);
        let encoded_fallback = escape_generic(&fixture);
        assert_eq!(encoded, encoded_fallback);
    }
}

fn get_rxjs_sources() -> Vec<String> {
    let dir = glob::glob("node_modules/rxjs/src/**/*.ts").unwrap();
    let mut sources = Vec::new();
    for entry in dir {
        sources.push(fs::read_to_string(entry.unwrap()).unwrap());
    }
    assert!(!sources.is_empty());
    sources
}
