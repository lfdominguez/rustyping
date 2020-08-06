use crate::redis::Host;
use fastping_rs::PingResult::{Idle, Receive};
use fastping_rs::Pinger;
use slog::Logger;
use std::collections::HashMap;

pub fn do_ping(
    ping_thread_log: &Logger,
    historical_hosts: &mut HashMap<String, String>,
    hosts: Vec<Host>,
    hostname: &str,
) {
    let local_inet_address = crate::utils::ifaddrs(ping_thread_log);

    let config_reader = crate::global_config::SETTINGS.read().unwrap();

    let timeout: f64 = config_reader.get("ping_timeout").unwrap();

    let (pinger, results) = match Pinger::new(Some(timeout as u64 * 1000), Some(16 as i32)) {
        Ok((pinger, results)) => (pinger, results),
        Err(e) => {
            error!(ping_thread_log, "Error creating pinger: {}", e);

            return;
        }
    };

    let mut deleted_hosts = vec![];

    for historical_host in historical_hosts.clone() {
        if !hosts.contains(&Host {
            address: historical_host.clone().0,
            name: historical_host.clone().1,
        }) || local_inet_address.contains(&historical_host.clone().0)
        {
            info!(ping_thread_log, "Removing host: {}", historical_host.0);
            deleted_hosts.push(historical_host);
        }
    }

    historical_hosts.retain(|key, value| !deleted_hosts.contains(&(key.clone(), value.clone())));

    for deleted_host in deleted_hosts {
        crate::prom::PING_LATENCY_HISTOGRAM
            .remove_label_values(&[&deleted_host.0[..], &deleted_host.1[..], hostname])
            .unwrap_or_default();
        crate::prom::PING_ERROR_COUNT
            .remove_label_values(&[&deleted_host.0[..], &deleted_host.1[..], hostname])
            .unwrap_or_default();
        crate::prom::PING_LAST
            .remove_label_values(&[&deleted_host.0[..], &deleted_host.1[..], hostname])
            .unwrap_or_default();
        crate::prom::HOST_UP
            .remove_label_values(&[&deleted_host.0[..], &deleted_host.1[..], hostname])
            .unwrap_or_default();
    }

    let mut can_ping = false;
    let mut expected_results = 0;

    for host in hosts {
        if !local_inet_address.contains(&host.address) {
            can_ping = true;
            expected_results += 1;
            pinger.add_ipaddr(&host.address[..]);
            historical_hosts
                .insert(host.address.clone(), host.name.clone())
                .or_else(|| {
                    crate::prom::PING_ERROR_COUNT
                        .with_label_values(&[
                            &host.address[..],
                            &historical_hosts[&host.address][..],
                            &hostname[..],
                        ])
                        .reset();
                    crate::prom::HOST_UP
                        .with_label_values(&[
                            &host.address[..],
                            &historical_hosts[&host.address][..],
                            &hostname[..],
                        ])
                        .set(0.0);

                    Some(host.name)
                });
        }
    }

    if can_ping {
        pinger.ping_once();

        for _ in 0..expected_results {
            match results.recv() {
                Ok(result) => match result {
                    Idle { addr } => {
                        let addr_str = addr.to_string();

                        crate::prom::PING_ERROR_COUNT
                            .with_label_values(&[
                                &addr_str[..],
                                &historical_hosts[&addr_str][..],
                                hostname,
                            ])
                            .inc();
                        crate::prom::PING_LAST
                            .remove_label_values(&[
                                &addr_str[..],
                                &historical_hosts[&addr_str][..],
                                hostname,
                            ])
                            .unwrap_or_default();
                        crate::prom::HOST_UP
                            .with_label_values(&[
                                &addr_str[..],
                                &historical_hosts[&addr_str][..],
                                hostname,
                            ])
                            .set(0.0);
                        warn!(ping_thread_log, "No response from host"; "host" => addr_str);
                    }
                    Receive { addr, rtt } => {
                        let addr_str = addr.to_string();
                        let millis = rtt.as_millis();

                        crate::prom::PING_LATENCY_HISTOGRAM
                            .with_label_values(&[
                                &addr_str[..],
                                &historical_hosts[&addr_str][..],
                                hostname,
                            ])
                            .observe(millis as f64);
                        crate::prom::PING_LAST
                            .with_label_values(&[
                                &addr_str[..],
                                &historical_hosts[&addr_str][..],
                                hostname,
                            ])
                            .set(millis as f64);
                        crate::prom::HOST_UP
                            .with_label_values(&[
                                &addr_str[..],
                                &historical_hosts[&addr_str][..],
                                hostname,
                            ])
                            .set(1.0);

                        trace!(
                            ping_thread_log,
                            "Response from {} latency {} ms",
                            addr_str,
                            millis
                        );
                    }
                },
                Err(_) => panic!("Worker threads disconnected before the solution was found!"),
            }
        }
    } else {
        warn!(ping_thread_log, "No host for ICMP request");
    }
}
