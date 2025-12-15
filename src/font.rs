pub fn load_font_from_file(path: &str) -> anyhow::Result<Vec<u8>> {
    std::fs::read(path).map_err(|e| anyhow::anyhow!("Failed to load font from {}: {}", path, e))
}

pub fn load_font_from_bytes(bytes: &[u8]) -> Vec<u8> {
    bytes.to_vec()
}
