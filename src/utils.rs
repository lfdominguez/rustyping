use nix::ifaddrs::getifaddrs;
use nix::sys::socket::SockAddr::Inet;
use nix::unistd::gethostname;
use slog::Logger;

pub fn ifaddrs(ping_thread_log: Logger) -> Vec<String> {
    let mut local_inet_address = vec![];

    for ifaddr in getifaddrs().unwrap() {
        match ifaddr.address {
            Some(address) => match address {
                Inet(_) => {
                    let mut inet_address = address.to_string();

                    inet_address.truncate(inet_address.len() - 2);

                    trace!(
                        ping_thread_log,
                        "interface {} address {}",
                        ifaddr.interface_name,
                        inet_address
                    );
                    local_inet_address.push(inet_address);
                }
                _ => {}
            },
            None => {}
        }
    }

    return local_inet_address;
}

pub fn host_name() -> String {
    let mut buf = [0u8; 128];
    let hostname_cstr = gethostname(&mut buf).expect("Failed getting hostname");
    let hostname = hostname_cstr.to_str().expect("Hostname wasn't valid UTF-8");

    String::from(hostname)
}
