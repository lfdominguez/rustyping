use redis::Commands;
use slog::Logger;

#[derive(Debug, Clone)]
pub struct Host {
    pub name: String,
    pub address: String,
}

impl PartialEq for Host {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address
    }
}

pub fn get_redis_hosts(ping_thread_log: &Logger) -> Result<Vec<Host>, ()> {
    let redis_host = crate::global_config::SETTINGS
        .read()
        .unwrap()
        .get::<String>("redis_host")
        .unwrap();

    redis::Client::open(format!("redis://{}/", redis_host))
    .and_then(|client| {
        client.get_connection()
        .and_then(|mut redis_connection| {
            trace!(ping_thread_log, "Connection to redis success"; "redis_server" => "192.168.128.64");

            let mut ret_servers = vec![];
            ret_servers.push(Host {
                            name: "server_name".to_string(),
                            address: "server_ip".to_string(),
                        });

            let smembers: Result<Vec<String>, redis::RedisError> = redis_connection.smembers("rustyping:targets");

            match smembers {
                Ok(servers_set) => {
                    let mut ret_servers = vec![];

                    for server in servers_set {
                        trace!(ping_thread_log, "New server found"; "server" => server.clone());

                        let split = server.split('@').collect::<Vec<&str>>();

                        if split.len() > 1 {
                            let server_name = split[0];
                            let server_ip = split[1];

                            ret_servers.push(Host {
                                name: server_name.to_string(),
                                address: server_ip.to_string(),
                            });
                        }
                    }

                    Ok(ret_servers)
                }
                Err(error) => {
                    error!(ping_thread_log, "Error getting the IPs from Redis Set: {}", error);
                    Err(error)
                }
            }
        })
    })
    .map_err::<(), _>(|error| {
        error!(ping_thread_log, "Error on Redis connection: {}", error);
    })
}
