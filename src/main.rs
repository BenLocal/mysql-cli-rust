use clap::{Arg, Command};
use std::io::{self, Write};

mod cli;
mod commands;
mod completion;
mod database;

use cli::Cli;

fn main() -> anyhow::Result<()> {
    let matches = Command::new("mysql-cli-rust")
        .version("0.1.0")
        .about("A MySQL client CLI written in Rust")
        .arg(
            Arg::new("host")
                .long("host")
                .value_name("HOST")
                .help("Connect to host")
                .default_value("localhost"),
        )
        .arg(
            Arg::new("port")
                .short('P')
                .long("port")
                .value_name("PORT")
                .help("Port number to use for connection")
                .default_value("3306"),
        )
        .arg(
            Arg::new("user")
                .short('u')
                .long("user")
                .value_name("USER")
                .help("User for login if not current user")
                .required(true),
        )
        .arg(
            Arg::new("password")
                .short('p')
                .long("password")
                .value_name("PASSWORD")
                .help("Password to use when connecting to server")
                .num_args(0..=1)
                .require_equals(true),
        )
        .arg(
            Arg::new("database")
                .short('D')
                .long("database")
                .value_name("DATABASE")
                .help("Database to use"),
        )
        .get_matches();

    let host = matches.get_one::<String>("host").unwrap();
    let port: u16 = matches
        .get_one::<String>("port")
        .unwrap()
        .parse()
        .expect("Invalid port number");
    let user = matches.get_one::<String>("user").unwrap();

    let password = if matches.contains_id("password") {
        match matches.get_one::<String>("password") {
            Some(p) => p.clone(),
            None => {
                print!("Enter password: ");
                io::stdout().flush().unwrap();
                rpassword::read_password().unwrap_or_default()
            }
        }
    } else {
        print!("Enter password: ");
        io::stdout().flush().unwrap();
        rpassword::read_password().unwrap_or_default()
    };

    let database = matches.get_one::<String>("database").cloned();

    let mut cli = Cli::new(host, port, user, &password, database.as_deref())?;
    cli.run()?;

    Ok(())
}
