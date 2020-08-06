use hyper::{header::CONTENT_TYPE, header::LOCATION, Body, Request, Response};

use prometheus::{Encoder, TextEncoder};

use crate::prom;

pub async fn serve_req(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    prom::HTTP_COUNTER.inc();

    if req.uri() == "/metrics" {
        let timer = prom::HTTP_REQ_HISTOGRAM
            .with_label_values(&["all"])
            .start_timer();

        let metric_families = prometheus::gather();

        let mut buffer = vec![];

        let encoder = TextEncoder::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();

        let encoder = TextEncoder::new();
        encoder
            .encode(&prom::CUSTOM_REGISTRY.gather(), &mut buffer)
            .unwrap();

        let response = Response::builder()
            .status(200)
            .header(CONTENT_TYPE, encoder.format_type())
            .body(Body::from(buffer))
            .unwrap();
        timer.observe_duration();

        Ok(response)
    } else {
        let response = Response::builder()
            .status(301)
            .header(LOCATION, "/metrics")
            .body(Body::from(""))
            .unwrap();
        Ok(response)
    }
}
