mod codegen;
mod ir;
mod schema;
mod searcher;

use codegen::{Codegen, JSCodegen};
use searcher::Searcher;

fn main() -> Result<(), std::io::Error> {
    let s1_path = std::env::args().nth(1).expect("need first argument");
    let s2_path = std::env::args().nth(2).expect("need second argument");

    let s1_json: serde_json::Value =
        serde_json::from_str(std::fs::read_to_string(s1_path)?.as_str())
            .expect("first schema has valid JSON");
    let s2_json: serde_json::Value =
        serde_json::from_str(std::fs::read_to_string(s2_path)?.as_str())
            .expect("second schema has valid JSON");

    let s1 = schema::Schema::try_from(&s1_json).expect("first schema valid");
    let s2 = schema::Schema::try_from(&s2_json).expect("first schema valid");

    let mut schr = searcher::SchemaSearcher::new();
    let code = schr.find_path(&s1, &s2).and_then(|path| {
        let gen = JSCodegen::new("input".to_string(), "output".to_string());
        let code = gen.generate(path.into_iter());
        Ok(code)
    });
    match code {
        Ok(code) => println!("{}", code),
        Err(e) => eprintln!("Could not find transformer between schemas: {:?}", e)
    }
    Ok(())
}
