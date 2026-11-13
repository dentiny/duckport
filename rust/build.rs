use std::{env, fs, path::PathBuf};

fn main() {
    let directory = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("src/readers");
    println!("cargo:rerun-if-changed={}", directory.display());
    let mut files: Vec<_> = fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .collect();
    files.sort();
    let mut modules = String::new();
    let mut open = String::new();
    let mut decode = String::new();
    for path in files {
        let module = path.file_stem().unwrap().to_str().unwrap();
        let (category, name) = module
            .split_once('_')
            .expect("reader modules need a category prefix");
        assert!(matches!(category, "format" | "service"));
        modules += &format!("#[path = {:?}] mod {module};\n", path);
        if category == "format" {
            open += &format!(
                "{name:?} => {{ let (input, path) = open_file(locator, &options).await?; {module}::decode(input, &path, &options, read_options).await }},\n"
            );
            decode += &format!(
                "{name:?} => {module}::decode(input, path, options, read_options).await,\n"
            );
        } else {
            open += &format!("{name:?} => {module}::open(locator, options, read_options).await,\n");
        }
    }
    modules += &format!(
        "async fn open_reader(reader: &str, locator: &str, options: Options, read_options: ReadOptions) -> Result<Box<dyn RecordReader>> {{ match reader {{ {open} _ => Err(format!(\"unsupported reader: {{reader}}\").into()) }} }}\n"
    );
    modules += &format!(
        "#[allow(dead_code)]\nasync fn decode_format(format: &str, input: Box<dyn ByteReader>, path: &str, options: &Options, read_options: ReadOptions) -> Result<Box<dyn RecordReader>> {{ match format {{ {decode} _ => Err(format!(\"unsupported format: {{format}}\").into()) }} }}\n"
    );
    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("readers.rs"),
        modules,
    )
    .unwrap();
}
