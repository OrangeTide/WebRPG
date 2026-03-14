#![recursion_limit = "1024"]

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{LeptosRoutes, generate_route_list};
    use webrpg::app::*;

    dotenvy::dotenv().ok();

    // Initialize database pool
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env file");
    webrpg::db::init_pool(&database_url);

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    let routes = generate_route_list(App);

    let app = Router::new()
        .route(
            "/api/ws",
            axum::routing::get(webrpg::server::ws_handler::ws_upgrade),
        )
        .route(
            "/api/media/upload",
            axum::routing::post(webrpg::server::media_handler::upload_media),
        )
        .route(
            "/api/media/{hash}",
            axum::routing::get(webrpg::server::media_handler::serve_media),
        )
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    let tls_config = match (
        std::env::var("TLS_CERT_PATH"),
        std::env::var("TLS_KEY_PATH"),
    ) {
        (Ok(cert_path), Ok(key_path)) => Some(
            axum_server::tls_rustls::RustlsConfig::from_pem_file(&cert_path, &key_path)
                .await
                .expect("Failed to load TLS certificate/key"),
        ),
        _ => None,
    };

    if let Some(tls_config) = tls_config {
        let tls_port: u16 = std::env::var("TLS_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3443);
        let tls_addr = std::net::SocketAddr::from(([0, 0, 0, 0], tls_port));

        // HTTP server redirects all requests to HTTPS
        let redirect_app = Router::new().fallback(move |req: axum::extract::Request| async move {
            use axum::response::IntoResponse;
            let host = req
                .headers()
                .get(axum::http::header::HOST)
                .and_then(|h| h.to_str().ok())
                .unwrap_or("localhost");
            let hostname = host.split(':').next().unwrap_or("localhost");
            let path = req
                .uri()
                .path_and_query()
                .map(|pq| pq.as_str())
                .unwrap_or("/");
            let https_url = if tls_port == 443 {
                format!("https://{hostname}{path}")
            } else {
                format!("https://{hostname}:{tls_port}{path}")
            };
            axum::response::Redirect::temporary(&https_url).into_response()
        });

        log!("HTTPS on https://{}", tls_addr);
        log!("HTTP redirect on http://{}", addr);

        let https_handle = tokio::spawn(async move {
            axum_server::bind_rustls(tls_addr, tls_config)
                .serve(app.into_make_service())
                .await
                .unwrap();
        });

        let http_handle = tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
            axum::serve(listener, redirect_app.into_make_service())
                .await
                .unwrap();
        });

        tokio::select! {
            r = https_handle => r.unwrap(),
            r = http_handle => r.unwrap(),
        }
    } else {
        // No built-in TLS — apply middleware that checks X-Forwarded-Proto
        // for reverse proxy deployments
        let app = app.layer(axum::middleware::from_fn(require_https_for_auth));

        log!("listening on http://{}", &addr);
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    }
}

/// Middleware that redirects /login to HTTPS when behind a reverse proxy
/// that sets X-Forwarded-Proto. No-op when the header is absent (plain HTTP dev).
#[cfg(feature = "ssr")]
async fn require_https_for_auth(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let path = req.uri().path();
    if path.starts_with("/login") {
        if let Some(proto) = req.headers().get("x-forwarded-proto") {
            if proto.as_bytes() != b"https" {
                let host = req
                    .headers()
                    .get(axum::http::header::HOST)
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or("localhost");
                let pq = req
                    .uri()
                    .path_and_query()
                    .map(|pq| pq.as_str())
                    .unwrap_or("/login");
                let url = format!("https://{host}{pq}");
                return axum::response::Redirect::temporary(&url).into_response();
            }
        }
    }
    next.run(req).await
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // see lib.rs for hydration function instead
}
