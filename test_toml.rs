fn main() {
    let mut doc = "[config]\n".parse::<toml_edit::DocumentMut>().unwrap();
    doc["config.nvim"] = toml_edit::Item::Table(toml_edit::Table::new());
    println!("{}", doc.to_string());
}
