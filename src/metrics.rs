use std::sync::LazyLock;

use prometheus::{
    CounterVec, HistogramVec, IntGauge, TextEncoder, register_counter_vec, register_histogram_vec,
    register_int_gauge,
};

static HTTP_REQUESTS_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
    register_counter_vec!("http_requests_total", "Total HTTP requests", &[
        "method", "path", "status"
    ])
    .expect("Failed to register counter")
});

static HTTP_ACTIVE_REQUESTS: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!(
        "http_active_requests",
        "Number of requests currently being processed",
    )
    .expect("Failed to register active requests gauge")
});

static HTTP_REQUEST_DURATION: LazyLock<HistogramVec> = LazyLock::new(|| {
    register_histogram_vec!(
        "http_request_duration_seconds",
        "HTTP request duration",
        &["method", "path"],
        vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5]
    )
    .expect("Failed to register histogram")
});

/// Record a completed request: increment counter and observe duration.
#[inline]
pub fn record(method: &str, path: &str, status: u16, duration_secs: f64) {
    let status_group = format!("{}xx", status / 100);
    HTTP_REQUESTS_TOTAL
        .with_label_values(&[method, path, &status_group])
        .inc();
    HTTP_REQUEST_DURATION
        .with_label_values(&[method, path])
        .observe(duration_secs);
}

/// Increment the active-requests gauge.
#[inline]
pub fn inc_active() {
    HTTP_ACTIVE_REQUESTS.inc();
}

/// Decrement the active-requests gauge.
#[inline]
pub fn dec_active() {
    HTTP_ACTIVE_REQUESTS.dec();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_requests_gauge_tracks_inc_and_dec() {
        // Force-init the gauge (LazyLock) by calling inc then dec first.
        inc_active();
        dec_active();

        // Initial state: 0 active requests
        let before = render();
        assert!(
            before.contains("http_active_requests 0"),
            "expected 0 active requests before test, got:\n{before}",
        );

        inc_active();
        let during = render();
        assert!(
            during.contains("http_active_requests 1"),
            "expected 1 active request after inc, got:\n{during}",
        );

        dec_active();
        let after = render();
        assert!(
            after.contains("http_active_requests 0"),
            "expected 0 active requests after dec, got:\n{after}",
        );
    }
}
