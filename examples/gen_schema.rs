use homeassistant_agent::connector::ConnectorOptions;

fn main() -> anyhow::Result<()> {
    let schema = schemars::schema_for!(ConnectorOptions);
    let path = "schema/connector.json";

    let file = std::fs::File::create(path)?;
    serde_json::to_writer_pretty(file, &schema)?;

    println!("Wrote schema to: {path}");

    Ok(())
}
