use crate::utils::get_installed_packages;
use ansi_term::Colour::Green;

pub fn list() {
    println!("{}", Green.paint("Installed packages:"));
    let packages = get_installed_packages();
    if packages.is_empty() {
        println!("No packages installed");
    } else {
        for pkg in packages {
            println!("- {} ({})", pkg.name, pkg.build_system);
        }
    }
}
