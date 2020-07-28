#[macro_use]
extern crate prometheus;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate slog;
extern crate slog_term;

use hyper::{
    service::{make_service_fn, service_fn},
    Server,
};

use std::thread;
use slog::Drain;

mod utils;
mod redis;
mod prom;
mod web;
mod task;
mod app_flags;
mod global_config;

#[tokio::main]
async fn main() {
    prom::init_custom_registry();

    let _guard = slog_envlogger::init().unwrap();

    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator)
        .build()
        .fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    let root_log = slog::Logger::root(drain, o!());
    let ping_thread_log = root_log.new(o!("component" => "ping_thread"));


    // Configure Flags
    match app_flags::configure() {
        Ok(()) => {}
        Err(error) => {
            error!(root_log, "Error on application flags: {}", error);
            return;
        }
    }

    // Dedicated thread to ping work
    thread::spawn(move || {
        let mut historical_hosts= std::collections::HashMap::<String, String>::new();
        let hostname = utils::host_name();

        let config_reader = global_config::SETTINGS.read().unwrap();

        loop {
            redis::get_redis_hosts(ping_thread_log.clone())
            .and_then(|hosts| {
                task::do_ping_task(ping_thread_log.clone(), &mut historical_hosts, hosts, hostname.clone());

                Ok(())
            })
            .unwrap_or_default();

            thread::sleep(std::time::Duration::from_secs(config_reader.get("ping_interval").unwrap()));
        }
    });

    // Start the Web server

    let addr = ([0, 0, 0, 0], 9898).into();

    let serve_future = Server::bind(&addr).serve(make_service_fn(|_| async {
        Ok::<_, hyper::Error>(service_fn(web::serve_req))
    }));

    info!(root_log, "Listening on http://{}", addr);

    if let Err(err) = serve_future.await {
        error!(root_log, "server error: {}", err);
    }
}
