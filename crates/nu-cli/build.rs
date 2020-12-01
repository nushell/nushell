fn main() -> shadow_rs::SdResult<()> {
    let src_path = std::env::var("CARGO_MANIFEST_DIR")?;
    let out_path = std::env::var("OUT_DIR")?;
    shadow_rs::Shadow::build(src_path, out_path)?;
    Ok(())
}
