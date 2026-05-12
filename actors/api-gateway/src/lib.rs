//! API Gateway actor — compiled to wasm32-wasip2 via `wash build`.
//!
//! Minimal MVP: serve health check at GET /api/v1/health.
//! Full implementation (routing, OAuth, UI serving) will be added incrementally.

wit_bindgen::generate!({
    world: "api-gateway",
    generate_all,
});

struct Component;

impl exports::wasi::http::incoming_handler::Guest for Component {
    fn handle(
        request: wasi::http::types::IncomingRequest,
        response_out: wasi::http::types::ResponseOutparam,
    ) {
        let path = request.path_with_query().unwrap_or_default();

        // Health check endpoint
        if path.starts_with("/api/v1/health") {
            send_response(response_out, 200)
        } else {
            // Not found
            send_response(response_out, 404)
        }
    }
}

fn send_response(response_out: wasi::http::types::ResponseOutparam, status: u16) {
    let headers = wasi::http::types::Fields::new();
    let resp = wasi::http::types::OutgoingResponse::new(headers);
    let _ = resp.set_status_code(status);

    let body = match resp.body() {
        Ok(b) => b,
        Err(_) => {
            return;
        }
    };
    let _ = wasi::http::types::OutgoingBody::finish(body, None);

    wasi::http::types::ResponseOutparam::set(response_out, Ok(resp));
}

export!(Component);
