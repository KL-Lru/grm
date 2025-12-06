use crate::configs::Config;
use crate::errors::GrmError;

pub fn execute() -> Result<(), GrmError> {
    let config = Config::load()?;
    println!("{}", config.root().display());
    Ok(())
}
