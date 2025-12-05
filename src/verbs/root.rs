use crate::configs::Config;

pub fn execute() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load()?;
    println!("{}", config.root().display());
    Ok(())
}
