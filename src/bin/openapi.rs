fn main() -> anyhow::Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(&kit::http::openapi_document())?
    );
    Ok(())
}
