//! Which browsers may open the WebSocket (ported from caravan's `origin.rs`). steerd listens on
//! loopback, but any web page the user visits can open `ws://127.0.0.1:<port>/ws` (CORS does not
//! apply to WebSockets), and a DNS-rebinding page can reach it under its own host name. Either
//! could read the frames and drive the sim. So the upgrade is
//! accepted only when `Host` names this loopback listener and `Origin`, if any, is a page this
//! host serves (or the Vite dev server). Clients without `Origin` are not browsers and cannot be
//! driven by a web page.

use axum::http::{header, HeaderMap};

/// `npm run dev` serves the web app here and proxies `/ws` to steerd.
const VITE_DEV_PORT: u16 = 5173;

#[derive(Debug, Clone, Copy)]
pub struct LoopbackGuard {
    pub port: u16,
}

impl LoopbackGuard {
    pub fn allows(&self, headers: &HeaderMap) -> bool {
        let host = headers.get(header::HOST).and_then(|v| v.to_str().ok());
        if !host.is_some_and(|h| self.is_loopback_authority(h)) {
            return false;
        }
        match headers.get(header::ORIGIN) {
            None => true,
            Some(origin) => origin
                .to_str()
                .ok()
                .and_then(|o| o.strip_prefix("http://"))
                .is_some_and(|authority| self.is_loopback_authority(authority)),
        }
    }

    /// `127.0.0.1:<port>`, `localhost:<port>` or `[::1]:<port>` for this host or the dev server.
    fn is_loopback_authority(&self, authority: &str) -> bool {
        let Some((name, port)) = authority.rsplit_once(':') else {
            return false;
        };
        let port_ok = port
            .parse::<u16>()
            .is_ok_and(|p| p == self.port || p == VITE_DEV_PORT);
        port_ok && matches!(name, "127.0.0.1" | "localhost" | "[::1]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers(host: Option<&str>, origin: Option<&str>) -> HeaderMap {
        let mut h = HeaderMap::new();
        if let Some(v) = host {
            h.insert(header::HOST, HeaderValue::from_str(v).unwrap());
        }
        if let Some(v) = origin {
            h.insert(header::ORIGIN, HeaderValue::from_str(v).unwrap());
        }
        h
    }

    const GUARD: LoopbackGuard = LoopbackGuard { port: 7400 };

    #[test]
    fn the_served_page_and_the_dev_server_are_allowed() {
        assert!(GUARD.allows(&headers(
            Some("127.0.0.1:7400"),
            Some("http://127.0.0.1:7400")
        )));
        assert!(GUARD.allows(&headers(
            Some("localhost:7400"),
            Some("http://localhost:7400")
        )));
        assert!(GUARD.allows(&headers(
            Some("localhost:5173"),
            Some("http://localhost:5173")
        )));
    }

    #[test]
    fn a_non_browser_client_without_origin_is_allowed() {
        assert!(GUARD.allows(&headers(Some("127.0.0.1:7400"), None)));
    }

    #[test]
    fn another_site_is_refused() {
        assert!(!GUARD.allows(&headers(
            Some("127.0.0.1:7400"),
            Some("https://evil.example")
        )));
        assert!(!GUARD.allows(&headers(
            Some("127.0.0.1:7400"),
            Some("http://127.0.0.1:8080")
        )));
        assert!(!GUARD.allows(&headers(Some("127.0.0.1:7400"), Some("null"))));
    }

    /// A DNS-rebinding page reaches 127.0.0.1 under its own name, so Host gives it away.
    #[test]
    fn a_rebound_host_name_is_refused() {
        assert!(!GUARD.allows(&headers(
            Some("evil.example:7400"),
            Some("http://evil.example:7400")
        )));
        assert!(!GUARD.allows(&headers(Some("evil.example:7400"), None)));
        assert!(!GUARD.allows(&headers(None, None)));
    }
}
