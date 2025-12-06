use crate::{configs::Config, discovery::scan_repositories, errors::GrmError};

/// Execute the list command
///
/// Lists all managed repositories under the configured root directory.
/// Repositories are displayed in alphabetical order.
///
/// # Arguments
/// * `full_path` - If true, shows absolute paths; otherwise shows relative paths from root
///
/// # Returns
/// * `Ok(())` on success
/// * `Err` if configuration loading or directory scanning fails
pub fn execute(full_path: bool) -> Result<(), GrmError> {
    let config = Config::load()?;
    let root = config.root();

    if !root.exists() {
        println!("Nothing to display");
        return Ok(());
    }

    let mut repositories = scan_repositories(root);

    if repositories.is_empty() {
        println!("Nothing to display");
        return Ok(());
    }

    repositories.sort();

    for repo in repositories {
        if full_path {
            println!("{}", repo.display());
        } else {
            match repo.strip_prefix(root) {
                Ok(relative) => println!("{}", relative.display()),
                Err(_) => println!("{}", repo.display()),
            }
        }
    }

    Ok(())
}
