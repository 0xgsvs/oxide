use prometheus::{
    CounterVec, HistogramVec, TextEncoder, register_counter_vec, register_histogram_vec,
};

static HTTP_REQUESTS_TOTAL: once_cell::sync::Lazy<CounterVec> = once_cell::sync::Lazy::new(|| {
    register_counter_vec!("http_requests_total", "Total HTTP requests", &[
        "method", "path", "status"
    ])
    .expect("Failed to register counter")
});

static HTTP_REQUEST_DURATION: once_cell::sync::Lazy<HistogramVec> =
    once_cell::sync::Lazy::new(|| {
        register_histogram_vec!(
            "http_request_duration_seconds",
            "HTTP request duration",
            &["method", "path"],
            vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5]
        )
        .expect("Failed to register histogram")
    });

/// Record a completed request: increment counter and observe duration.
pub fn record(method: &str, path: &str, status: u16, duration_secs: f64) {
    let status_group = format!("{}xx", status / 100);
    HTTP_REQUESTS_TOTAL
        .with_label_values(&[method, path, &status_group])
        .inc();
    HTTP_REQUEST_DURATION
        .with_label_values(&[method, path])
        .observe(duration_secs);
}

/// Render all metrics as Prometheus text format.
pub fn render() -> String {
    let encoder = TextEncoder::new();
    let mut buffer = String::new();
    encoder
        .encode_utf8(&prometheus::gather(), &mut buffer)
        .expect("Failed to encode metrics");
    buffer
}
