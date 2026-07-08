fn main() {
    let mut doc = toml_edit::DocumentMut::new();
    
    // Create [config] table
    let mut config = toml_edit::Table::new();
    config.set_implicit(true); // if it's implicit, it doesn't print [config]? Let's see
    
    let mut nvim = toml_edit::Table::new();
    nvim.insert("ignores", toml_edit::value(toml_edit::Array::new()));
    
    config.insert("nvim", toml_edit::Item::Table(nvim));
    doc.insert("config", toml_edit::Item::Table(config));
    
    println!("DOC1:\n{}", doc.to_string());
}
