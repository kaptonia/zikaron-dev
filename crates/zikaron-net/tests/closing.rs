//! Shutdown (the app quitting) cuts an exchange in flight at once and refuses any started afterwards, both with
//! `Fail::Closed`; reopening lets exchanges through. Its own test binary, since the shutdown flag is
//! process-wide.

use std::io::Read;
use std::net::TcpListener;
use std::time::{Duration, Instant};
use zikaron_net::{close_down, open_up, parse, post_json, set_choice, Ask, Choice, Fail, Kind, Limits, Proxy};

#[test]
fn closing_down_cuts_what_is_in_flight_and_refuses_what_comes_after() {
    let l = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = l.local_addr().expect("addr").port();
    let held = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let h = held.clone();
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            let h = h.clone();
            std::thread::spawn(move || {
                let mut s = s;
                let mut buf = [0u8; 4096];
                let _ = s.read(&mut buf);
                h.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                // Never answers: only shutdown ends the exchange before its deadline.
                std::thread::sleep(Duration::from_secs(30));
            });
        }
    });
    let t = parse(&format!("http://127.0.0.1:{port}/")).expect("address");
    let limits = Limits { deadline: Duration::from_secs(20), max_answer: 0 };
    let asking = std::thread::spawn(move || {
        let began = Instant::now();
        (post_json(&t, b"{}", &limits, Ask::Read).err(), began.elapsed())
    });
    while held.load(std::sync::atomic::Ordering::SeqCst) == 0 {
        std::thread::sleep(Duration::from_millis(5));
    }
    close_down();
    let (said, took) = asking.join().expect("the asking thread");
    assert_eq!(said, Some(Fail::Closed));
    assert!(took < Duration::from_secs(5), "cut at once, not at the deadline: {took:?}");
    // Not retried after the cut: the peer saw one request.
    assert_eq!(held.load(std::sync::atomic::Ordering::SeqCst), 1);
    let t = parse(&format!("http://127.0.0.1:{port}/")).expect("address");
    assert_eq!(post_json(&t, b"{}", &limits, Ask::Read).err(), Some(Fail::Closed), "an exchange after closing down never opens");
    assert_eq!(held.load(std::sync::atomic::Ordering::SeqCst), 1);
    open_up();
    let closed = TcpListener::bind("127.0.0.1:0").expect("bind").local_addr().expect("addr").port();
    let t = parse(&format!("http://127.0.0.1:{closed}/")).expect("address");
    assert!(matches!(post_json(&t, b"{}", &limits, Ask::Read), Err(Fail::Connect(_))), "open again: the exchange goes out");

    // A handshake in flight (the peer accepted and stays silent) is also cut at once.
    let l = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = l.local_addr().expect("addr").port();
    let took_it = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let k = took_it.clone();
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            k.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            std::thread::spawn(move || {
                let mut s = s;
                let mut buf = [0u8; 4096];
                while matches!(s.read(&mut buf), Ok(n) if n > 0) {}
            });
        }
    });
    let t = parse(&format!("https://127.0.0.1:{port}/")).expect("address");
    let asking = std::thread::spawn(move || {
        let began = Instant::now();
        (post_json(&t, b"{}", &limits, Ask::Read).err(), began.elapsed())
    });
    while took_it.load(std::sync::atomic::Ordering::SeqCst) == 0 {
        std::thread::sleep(Duration::from_millis(5));
    }
    std::thread::sleep(Duration::from_millis(50));
    close_down();
    let (said, took) = asking.join().expect("the asking thread");
    assert_eq!(said, Some(Fail::Closed), "a handshake cut by closing down is said as that");
    assert!(took < Duration::from_secs(5), "cut at once, not at the deadline: {took:?}");
    open_up();

    // A tunnel in flight (the proxy accepted and stays silent) is also cut at once.
    let l = TcpListener::bind("127.0.0.1:0").expect("bind");
    let proxy = l.local_addr().expect("addr").port();
    let taken = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let k = taken.clone();
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            k.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            std::thread::spawn(move || {
                let mut s = s;
                let mut buf = [0u8; 4096];
                while matches!(s.read(&mut buf), Ok(n) if n > 0) {}
            });
        }
    });
    set_choice(Choice::Manual(Proxy { kind: Kind::Http, host: "127.0.0.1".into(), port: proxy }));
    let t = parse("http://node.invalid:8545/").expect("address");
    let asking = std::thread::spawn(move || {
        let began = Instant::now();
        (post_json(&t, b"{}", &limits, Ask::Read).err(), began.elapsed())
    });
    while taken.load(std::sync::atomic::Ordering::SeqCst) == 0 {
        std::thread::sleep(Duration::from_millis(5));
    }
    std::thread::sleep(Duration::from_millis(50));
    close_down();
    let (said, took) = asking.join().expect("the asking thread");
    assert_eq!(said, Some(Fail::Closed), "a tunnel cut by closing down is said as that");
    assert!(took < Duration::from_secs(5), "cut at once, not at the deadline: {took:?}");
    open_up();
    set_choice(Choice::Off);
}
