use prometheus::{
    exponential_buckets, Counter, CounterVec, GaugeVec, HistogramOpts, HistogramVec, Registry,
};

lazy_static! {
    pub static ref HTTP_COUNTER: Counter = register_counter!(opts!(
        "http_requests_total",
        "Total number of HTTP requests made.",
        labels! {"handler" => "all",}
    ))
    .unwrap();
    pub static ref HTTP_REQ_HISTOGRAM: HistogramVec = register_histogram_vec!(
        "http_request_duration_seconds",
        "The HTTP request latencies in seconds.",
        &["handler"]
    )
    .unwrap();
    pub static ref PING_LATENCY_HISTOGRAM: HistogramVec = HistogramVec::new(
        HistogramOpts::new("rustyping_ping_latency", "Ping Latency")
            .buckets(exponential_buckets(0.100, 1.37, 30).unwrap()),
        &["ip", "name", "from"],
    )
    .unwrap();
    pub static ref PING_ERROR_COUNT: CounterVec = CounterVec::new(
        opts!("rustyping_ping_error", "Ping error"),
        &["ip", "name", "from"]
    )
    .unwrap();
    pub static ref PING_LAST: GaugeVec = GaugeVec::new(
        opts!("rustyping_ping_last", "Last ping latency"),
        &["ip", "name", "from"]
    )
    .unwrap();
    pub static ref HOST_UP: GaugeVec = GaugeVec::new(
        opts!("rustyping_host_up", "Host is responding to ICMP"),
        &["ip", "name", "from"]
    )
    .unwrap();
    pub static ref CUSTOM_REGISTRY: Registry = Registry::new();
}

pub fn init_custom_registry() {
    CUSTOM_REGISTRY
        .register(Box::new(PING_LATENCY_HISTOGRAM.clone()))
        .unwrap();
    CUSTOM_REGISTRY
        .register(Box::new(PING_ERROR_COUNT.clone()))
        .unwrap();
    CUSTOM_REGISTRY
        .register(Box::new(PING_LAST.clone()))
        .unwrap();
    CUSTOM_REGISTRY.register(Box::new(HOST_UP.clone())).unwrap();
}
