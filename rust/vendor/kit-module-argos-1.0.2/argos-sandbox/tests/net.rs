//! The network service: HTTPS to allowed domains only, anti-SSRF checks on
//! resolved addresses and redirects. No test reaches the internet.

use std::net::{IpAddr, SocketAddr, TcpListener};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use argos_sandbox::net::{is_public, Lookup, NetService};
use argos_sandbox::Capability;
use argos_sandbox::{
    CallToken, HostServices, HOST_DENIED, HOST_FORBIDDEN_NET, HOST_IO, HOST_MALFORMED,
};

fn service() -> NetService {
    NetService::new(vec!["api.example.org".into()])
}

/// A lookup that answers `address` for any host and counts calls.
fn fixed(address: SocketAddr, count: Arc<AtomicUsize>) -> Lookup {
    Arc::new(move |_host: &str| {
        count.fetch_add(1, Ordering::SeqCst);
        Ok(vec![address])
    })
}

fn call(s: &NetService, request: &str) -> Result<Vec<u8>, i32> {
    s.call(
        Capability::Network,
        request.as_bytes(),
        &CallToken::default(),
    )
}

#[test]
fn only_https_to_an_allowed_domain_passes_the_url_check() {
    let s = service();
    assert!(s.check_url("https://api.example.org/v1/data").is_ok());
    assert!(s.check_url("https://API.example.org:443/v1").is_ok());
    for bad in [
        "http://api.example.org/",
        "ftp://api.example.org/",
        "https://other.example.org/",
        "https://example.org/",
        "https://evil.api.example.org/",
        "https://api.example.org:8443/",
        "https://user:pass@api.example.org/",
        "https://127.0.0.1/",
        "https://[::1]/",
        "https://10.0.0.1/",
        "https://localhost/",
        "https://api.localhost/",
    ] {
        assert_eq!(
            s.check_url(bad).map(|_| ()),
            Err(HOST_FORBIDDEN_NET),
            "{bad}"
        );
    }
    assert_eq!(s.check_url("not a url").map(|_| ()), Err(HOST_MALFORMED));
}

#[test]
fn local_private_and_reserved_addresses_are_not_public() {
    for ip in [
        "127.0.0.1",
        "10.1.2.3",
        "172.16.0.1",
        "192.168.1.1",
        "169.254.169.254",
        "100.64.0.1",
        "0.0.0.0",
        "255.255.255.255",
        "224.0.0.1",
        "192.0.2.1",
        "198.18.0.1",
        "240.0.0.1",
        "::1",
        "::",
        "fc00::1",
        "fd12::1",
        "fe80::1",
        "ff02::1",
        "2001:db8::1",
        "::ffff:127.0.0.1",
        "::ffff:10.0.0.1",
        "::127.0.0.1",
    ] {
        let ip: IpAddr = ip.parse().expect("ip");
        assert!(!is_public(ip), "{ip} counted as public");
    }
    for ip in ["93.184.216.34", "1.1.1.1", "2606:4700:4700::1111"] {
        let ip: IpAddr = ip.parse().expect("ip");
        assert!(is_public(ip), "{ip} counted as private");
    }
}

#[test]
fn a_domain_resolving_to_a_local_address_is_refused_before_connecting() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    listener.set_nonblocking(true).expect("nonblocking");
    let local = listener.local_addr().expect("addr");
    let count = Arc::new(AtomicUsize::new(0));
    let s = NetService::with_lookup(
        vec!["api.example.org".into()],
        fixed(local, Arc::clone(&count)),
    );
    assert_eq!(
        call(&s, "GET https://api.example.org/"),
        Err(HOST_FORBIDDEN_NET)
    );
    assert!(count.load(Ordering::SeqCst) >= 1, "the resolver was asked");
    assert!(
        listener.accept().is_err(),
        "a connection reached the local listener"
    );

    // A private address, or a public one mixed with a private one.
    for addresses in [
        vec!["10.0.0.5:443"],
        vec!["93.184.216.34:443", "192.168.0.10:443"],
    ] {
        let addresses: Vec<SocketAddr> =
            addresses.iter().map(|a| a.parse().expect("addr")).collect();
        let lookup: Lookup = Arc::new(move |_host: &str| Ok(addresses.clone()));
        let s = NetService::with_lookup(vec!["api.example.org".into()], lookup);
        assert_eq!(
            call(&s, "GET https://api.example.org/"),
            Err(HOST_FORBIDDEN_NET)
        );
    }
}

#[test]
fn redirects_stay_on_the_same_domain_over_https() {
    let s = NetService::new(vec!["api.example.org".into(), "cdn.example.org".into()]);
    let current = s.check_url("https://api.example.org/a").expect("url");
    assert!(s.next_url(&current, "/b").is_ok());
    assert!(s.next_url(&current, "https://api.example.org/c").is_ok());
    for bad in [
        "https://cdn.example.org/x", // allowed domain, but another one
        "http://api.example.org/x",
        "https://127.0.0.1/x",
        "//evil.example/x",
        "https://evil.example/x",
    ] {
        assert!(s.next_url(&current, bad).is_err(), "{bad}");
    }
}

#[test]
fn malformed_requests_cancelled_calls_and_other_rights_are_refused() {
    let count = Arc::new(AtomicUsize::new(0));
    let s = NetService::with_lookup(
        vec!["api.example.org".into()],
        fixed(
            "93.184.216.34:443".parse().expect("addr"),
            Arc::clone(&count),
        ),
    );
    for bad in [
        "",
        "GET",
        "DELETE https://api.example.org/",
        "GEThttps://api.example.org/",
    ] {
        assert_eq!(call(&s, bad), Err(HOST_MALFORMED), "{bad:?}");
    }
    assert_eq!(
        s.call(Capability::Network, b"GET \xff", &CallToken::default()),
        Err(HOST_MALFORMED)
    );
    // Cancelled before sending: nothing is looked up or sent.
    let cancelled = CallToken::cancelled();
    assert_eq!(
        s.call(
            Capability::Network,
            b"GET https://api.example.org/",
            &cancelled
        ),
        Err(HOST_IO)
    );
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert_eq!(
        s.call(Capability::ReadFiles, b"x", &CallToken::default()),
        Err(HOST_DENIED)
    );
}
