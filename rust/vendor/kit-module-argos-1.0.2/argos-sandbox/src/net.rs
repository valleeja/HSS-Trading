//! `network_request`: HTTPS only, only to the domains of the module's
//! manifest, never to a local or private address.
//!
//! Request: `GET <url>` or `POST <url>`, a newline, then the body (POST).
//! Answer: HTTP status (2 bytes, big-endian), then the body.
//!
//! - The URL must be `https://<allowed domain>[:443]/...`: no IP literal, no
//!   `localhost`, no user info, no other port.
//! - Every address the domain resolves to must be public (anti-SSRF), checked
//!   at each connection by the resolver, so a DNS answer cannot point the
//!   module at the local network.
//! - Redirects are followed by hand, at most 3, only to the same domain over
//!   HTTPS; each one goes through the same checks.
//! - No proxy from the environment, no cookies, no credentials: the host never
//!   adds an authentication header, so a module never sees the app's API keys.
//! - Response limited to what fits the 64 KiB answer; 10 s per request at most
//!   (the sandbox's own per-call budget is usually shorter).
//! - The request is sent only if the call is not cancelled; a request already
//!   being sent cannot be recalled.

use std::fmt;
use std::io::{self, ErrorKind};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs as _};
use std::sync::Arc;
use std::time::Duration;

use argos_module_policy::Capability;
use ureq::config::Config;
use ureq::http::Uri;
use ureq::unversioned::resolver::{DefaultResolver, ResolvedSocketAddrs, Resolver};
use ureq::unversioned::transport::{DefaultConnector, NextTimeout};
use ureq::Agent;

use crate::{
    CallToken, HostServices, HOST_DENIED, HOST_FORBIDDEN_NET, HOST_IO, HOST_MALFORMED,
    HOST_TOO_LARGE, MAX_REQUEST_BYTES,
};

/// Most redirects followed.
const MAX_REDIRECTS: usize = 3;
/// Longest time for one request.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Looks up `host:port`; replaceable in tests.
pub type Lookup = Arc<dyn Fn(&str) -> io::Result<Vec<SocketAddr>> + Send + Sync>;

/// The network service of one module.
#[derive(Clone)]
pub struct NetService {
    domains: Vec<String>,
    agent: Agent,
}

impl fmt::Debug for NetService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NetService")
            .field("domains", &self.domains)
            .finish_non_exhaustive()
    }
}

impl NetService {
    /// A service allowed to reach exactly `domains` (from the manifest).
    pub fn new(domains: Vec<String>) -> Self {
        Self::with_lookup(
            domains,
            Arc::new(|host: &str| host.to_socket_addrs().map(Iterator::collect)),
        )
    }

    /// Same, with a custom address lookup (tests).
    pub fn with_lookup(domains: Vec<String>, lookup: Lookup) -> Self {
        let config = Config::builder()
            .https_only(true)
            .http_status_as_error(false)
            .max_redirects(0)
            .proxy(None)
            .user_agent("argos-module")
            .timeout_global(Some(REQUEST_TIMEOUT))
            .build();
        let resolver = PublicOnly { lookup };
        let agent = Agent::with_parts(config, DefaultConnector::new(), resolver);
        Self {
            domains: domains
                .into_iter()
                .map(|d| d.to_ascii_lowercase())
                .collect(),
            agent,
        }
    }

    /// Checks a URL against the rules; returns it parsed.
    ///
    /// # Errors
    /// [`HOST_FORBIDDEN_NET`] or [`HOST_MALFORMED`].
    pub fn check_url(&self, url: &str) -> Result<Uri, i32> {
        let uri: Uri = url.parse().map_err(|_| HOST_MALFORMED)?;
        if uri.scheme_str() != Some("https") {
            return Err(HOST_FORBIDDEN_NET);
        }
        let authority = uri.authority().ok_or(HOST_MALFORMED)?;
        if authority.as_str().contains('@') || authority.port_u16().is_some_and(|p| p != 443) {
            return Err(HOST_FORBIDDEN_NET);
        }
        let host = authority.host().to_ascii_lowercase();
        let literal = host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<IpAddr>()
            .is_ok();
        let local = host == "localhost" || host.ends_with(".localhost");
        if literal || local || !self.domains.contains(&host) {
            return Err(HOST_FORBIDDEN_NET);
        }
        Ok(uri)
    }

    fn request(&self, request: &[u8], token: &CallToken) -> Result<Vec<u8>, i32> {
        let (line, body) = match request.iter().position(|&b| b == b'\n') {
            Some(i) => (&request[..i], &request[i + 1..]),
            None => (request, &[][..]),
        };
        let line = std::str::from_utf8(line).map_err(|_| HOST_MALFORMED)?;
        let (method, url) = line.split_once(' ').ok_or(HOST_MALFORMED)?;
        if method != "GET" && method != "POST" {
            return Err(HOST_MALFORMED);
        }
        let mut uri = self.check_url(url.trim())?;
        // A sent request cannot be recalled: check the token just before.
        if token.is_cancelled() {
            return Err(HOST_IO);
        }
        for _ in 0..=MAX_REDIRECTS {
            let result = if method == "POST" {
                self.agent.post(uri.clone()).send(body)
            } else {
                self.agent.get(uri.clone()).call()
            };
            let mut response = result.map_err(|e| net_status(&e))?;
            let status = response.status();
            if status.is_redirection() {
                let location = response
                    .headers()
                    .get("location")
                    .and_then(|v| v.to_str().ok())
                    .ok_or(HOST_FORBIDDEN_NET)?;
                uri = self.next_url(&uri, location)?;
                if token.is_cancelled() {
                    return Err(HOST_IO);
                }
                continue;
            }
            let limit = u64::try_from(MAX_REQUEST_BYTES - 2).unwrap_or(0);
            let bytes = response
                .body_mut()
                .with_config()
                .limit(limit)
                .read_to_vec()
                .map_err(|e| net_status(&e))?;
            let mut answer = status.as_u16().to_be_bytes().to_vec();
            answer.extend_from_slice(&bytes);
            return Ok(answer);
        }
        Err(HOST_FORBIDDEN_NET)
    }

    /// The target of a redirect: same domain, HTTPS, same checks.
    ///
    /// # Errors
    /// [`HOST_FORBIDDEN_NET`] or [`HOST_MALFORMED`].
    pub fn next_url(&self, current: &Uri, location: &str) -> Result<Uri, i32> {
        let absolute = if location.starts_with('/') && !location.starts_with("//") {
            let authority = current.authority().ok_or(HOST_MALFORMED)?;
            format!("https://{authority}{location}")
        } else {
            location.to_owned()
        };
        let next = self.check_url(&absolute)?;
        let same = |u: &Uri| u.host().map(str::to_ascii_lowercase);
        if same(&next) != same(current) {
            return Err(HOST_FORBIDDEN_NET);
        }
        Ok(next)
    }
}

impl HostServices for NetService {
    fn call(
        &self,
        capability: Capability,
        request: &[u8],
        token: &CallToken,
    ) -> Result<Vec<u8>, i32> {
        match capability {
            Capability::Network => self.request(request, token),
            _ => Err(HOST_DENIED),
        }
    }
}

/// True for addresses a module may reach: public unicast only.
pub fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => public_v4(v4),
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => public_v4(v4),
            None => public_v6(v6),
        },
    }
}

fn public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    !(ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_multicast()
        || ip.is_documentation()
        || a == 0
        || (a == 100 && (64..128).contains(&b)) // carrier-grade NAT
        || (a == 198 && (b == 18 || b == 19)) // benchmarking
        || a >= 240) // reserved
}

fn public_v6(ip: Ipv6Addr) -> bool {
    let first = ip.segments()[0];
    !(ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || (first & 0xfe00) == 0xfc00 // unique local
        || (first & 0xffc0) == 0xfe80 // link-local
        || first == 0x2001 && ip.segments()[1] == 0x0db8 // documentation
        || ip.to_ipv4().is_some()) // IPv4-compatible forms
}

/// Resolver that refuses a host if any of its addresses is not public.
struct PublicOnly {
    lookup: Lookup,
}

impl fmt::Debug for PublicOnly {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PublicOnly")
    }
}

impl Resolver for PublicOnly {
    fn resolve(
        &self,
        uri: &Uri,
        _config: &Config,
        _timeout: NextTimeout,
    ) -> Result<ResolvedSocketAddrs, ureq::Error> {
        let (Some(scheme), Some(authority)) = (uri.scheme(), uri.authority()) else {
            return Err(ureq::Error::HostNotFound);
        };
        let target =
            DefaultResolver::host_and_port(scheme, authority).ok_or(ureq::Error::HostNotFound)?;
        let addresses = (self.lookup)(&target)?;
        if addresses.is_empty() {
            return Err(ureq::Error::HostNotFound);
        }
        if !addresses.iter().all(|a| is_public(a.ip())) {
            return Err(ureq::Error::Io(io::Error::new(
                ErrorKind::PermissionDenied,
                "argos: address not public",
            )));
        }
        let mut result = self.empty();
        for address in addresses.into_iter().take(16) {
            result.push(address);
        }
        Ok(result)
    }
}

fn net_status(error: &ureq::Error) -> i32 {
    match error {
        ureq::Error::Io(e) if e.kind() == ErrorKind::PermissionDenied => HOST_FORBIDDEN_NET,
        ureq::Error::RequireHttpsOnly(_) => HOST_FORBIDDEN_NET,
        ureq::Error::BodyExceedsLimit(_) => HOST_TOO_LARGE,
        _ => HOST_IO,
    }
}
