use crate::commands::install;
use ansi_term::Colour::Green;

pub fn bulk_install(packages: &[String], flags: &[String]) {
    for package in packages {
        println!("{}", Green.paint(&format!("Installing {}...", package)));
        install::install(package, None, false, None, None, flags);
    }
}
