use clap::{App, Arg};
use std::net::{Ipv4Addr, Ipv6Addr};

fn is_positive(v: String) -> Result<(), String> {
    if v.parse::<u64>().is_ok() {
        return Ok(());
    } else {
        if let Ok(f) = v.parse::<f64>() {
            if f > 0_f64 {
                return Ok(());
            }
        }
    }
    Err(format!("{} isn't a positive number", &*v))
}

pub fn configure() -> Result<(), String> {
    let flags = App::new(option_env!("CARGO_PKG_NAME").unwrap_or("UNKNOWN"))
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or("UNKNOWN"))
        .about(option_env!("CARGO_PKG_DESCRIPTION").unwrap_or("UNKNOWN"))
        .author(option_env!("CARGO_PKG_AUTHORS").unwrap_or("UNKNOWN"))
        .arg(
            Arg::with_name("redis_host")
                .short("r")
                .long("redis_host")
                .help("IP Address of Redis server")
                .required(true)
                .takes_value(true)
                .env("RUSTYPING_REDIS_HOST"),
        )
        .arg(
            Arg::with_name("ping_timeout")
                .short("t")
                .long("ping_timeout")
                .help("Set the timeout in seconds")
                .required(false)
                .takes_value(true)
                .default_value("1.0")
                .validator(is_positive)
                .env("RUSTYPING_PING_TIMEOUT"),
        )
        .arg(
            Arg::with_name("ping_interval")
                .short("i")
                .long("ping_interval")
                .help("Set the interval in seconds of each ping request")
                .required(false)
                .takes_value(true)
                .default_value("5")
                .validator(is_positive)
                .env("RUSTYPING_PING_INTERVAL"),
        )
        .get_matches();

    let redis_host_str = flags.value_of("redis_host").unwrap();

    let mut is_v4 = false;
    let mut is_v6 = false;

    redis_host_str
        .parse::<Ipv4Addr>()
        .and_then(|ipv4| {
            is_v4 = true;

            Ok(ipv4)
        })
        .unwrap_or(Ipv4Addr::new(127, 0, 0, 1));

    if !is_v4 {
        redis_host_str
            .parse::<Ipv6Addr>()
            .and_then(|ipv6| {
                is_v6 = true;

                Ok(ipv6)
            })
            .unwrap_or(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1));
    }

    if !is_v4 && !is_v6 {
        return Err("Wrong redis host address".to_string());
    }

    let timeout = flags
        .value_of("ping_timeout")
        .unwrap()
        .parse::<f64>()
        .unwrap_or(1.0);
    let interval = flags
        .value_of("ping_interval")
        .unwrap()
        .parse::<i64>()
        .unwrap_or(5);

    if interval as f64 <= timeout {
        return Err("The timeout must be bigger than interval".to_string());
    }

    let mut settings_writter = crate::global_config::SETTINGS.write().unwrap();

    settings_writter.set("redis_host", redis_host_str).unwrap();
    settings_writter.set("ping_timeout", timeout).unwrap();
    settings_writter.set("ping_interval", interval).unwrap();

    Ok(())
}
