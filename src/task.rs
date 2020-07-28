use slog::Logger;
use std::collections::HashMap;
use crate::redis::Host;

pub fn do_ping_task(ping_thread_log: Logger, historical_hosts: &mut HashMap<String, String>, hosts: Vec<Host>, hostname: String) {
    let local_inet_address = crate::utils::ifaddrs(ping_thread_log.clone());

    let mut ping = oping::Ping::new();


    let config_reader = crate::global_config::SETTINGS.read().unwrap();

    ping.set_timeout(config_reader.get("ping_timeout").unwrap()).unwrap();

    let mut deleted_hosts = vec![];

    for historical_host in historical_hosts.clone() {
        if !hosts.contains(&Host {
            address: historical_host.clone().0,
            name: historical_host.clone().1,
        }) || local_inet_address.contains(&historical_host.clone().0) {
            info!(ping_thread_log, "Removing host: {}", historical_host.0);
            deleted_hosts.push(historical_host);
        }
    }

    historical_hosts.retain(|key, value| {
        !deleted_hosts.contains(&(key.clone(), value.clone()))
    });

    for deleted_host in deleted_hosts {
        crate::prom::PING_LATENCY_HISTOGRAM.remove_label_values(&[&deleted_host.0[..], &deleted_host.1[..], &hostname[..]]).unwrap_or_default();
        crate::prom::PING_ERROR_COUNT.remove_label_values(&[&deleted_host.0[..], &deleted_host.1[..], &hostname[..]]).unwrap_or_default();
        crate::prom::PING_LAST.remove_label_values(&[&deleted_host.0[..], &deleted_host.1[..], &hostname[..]]).unwrap_or_default();
        crate::prom::HOST_UP.remove_label_values(&[&deleted_host.0[..], &deleted_host.1[..], &hostname[..]]).unwrap_or_default();
    }

    let mut can_ping = false;

    for host in hosts {
        if !local_inet_address.contains(&host.address) {
            can_ping = true;
            ping.add_host(&host.address[..]).unwrap();
            historical_hosts.insert(host.address.clone(), host.name.clone()).or_else(|| {
                crate::prom::PING_ERROR_COUNT.with_label_values(&[&host.address[..], &historical_hosts[&host.address][..], &hostname[..]]).reset();
                crate::prom::HOST_UP.with_label_values(&[&host.address[..], &historical_hosts[&host.address][..], &hostname[..]]).set(0.0);

                Some(host.name)
            });
        }
    }

    if can_ping {
        let responses = ping.send().unwrap();
        for resp in responses {
            if resp.dropped > 0 {
                crate::prom::PING_ERROR_COUNT.with_label_values(&[&resp.address[..], &historical_hosts[&resp.address][..], &hostname[..]]).inc();
                crate::prom::PING_LAST.remove_label_values(&[&resp.address[..], &historical_hosts[&resp.address][..], &hostname[..]]).unwrap_or_default();
                crate::prom::HOST_UP.with_label_values(&[&resp.address[..], &historical_hosts[&resp.address][..], &hostname[..]]).set(0.0);
                warn!(ping_thread_log, "No response from host"; "host" => resp.address);
            } else {            
                crate::prom::PING_LATENCY_HISTOGRAM.with_label_values(&[&resp.address[..], &historical_hosts[&resp.address][..], &hostname[..]]).observe(resp.latency_ms);
                crate::prom::PING_LAST.with_label_values(&[&resp.address[..], &historical_hosts[&resp.address][..], &hostname[..]]).set(resp.latency_ms);
                crate::prom::HOST_UP.with_label_values(&[&resp.address[..], &historical_hosts[&resp.address][..], &hostname[..]]).set(1.0);

                trace!(ping_thread_log, "Response from {} latency {} ms", resp.address, resp.latency_ms);
            }
        }
    } else {
        warn!(ping_thread_log, "No host for ICMP request");
    }
}