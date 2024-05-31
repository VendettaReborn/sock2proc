use criterion::{criterion_group, criterion_main, Criterion};
use sock2proc::{FindProc, FindProcImpl};

fn run_find_process_by_socket() {
    let dst = std::net::SocketAddr::new(
        std::net::IpAddr::V4(std::net::Ipv4Addr::new(8, 8, 8, 8)),
        80,
    );
    let _process_name = FindProcImpl::resolve(None, Some(dst), libc::IPPROTO_TCP);
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("find_process_by_socket", |b| {
        b.iter(|| run_find_process_by_socket())
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
