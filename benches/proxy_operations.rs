use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use proxy_spider::proxy::{Proxy, ProxyType};
use std::collections::HashSet;

fn create_test_proxy(i: usize) -> Proxy {
    Proxy {
        protocol: ProxyType::Http,
        host: format!("192.168.{}.{}", i / 256, i % 256),
        port: 8080,
        username: None,
        password: None,
        timeout: None,
        exit_ip: None,
    }
}

fn bench_proxy_to_string(c: &mut Criterion) {
    let proxy = create_test_proxy(1);
    
    c.bench_function("proxy_to_string_no_protocol", |b| {
        b.iter(|| {
            let s = black_box(&proxy).to_string(false);
            black_box(s);
        });
    });
    
    c.bench_function("proxy_to_string_with_protocol", |b| {
        b.iter(|| {
            let s = black_box(&proxy).to_string(true);
            black_box(s);
        });
    });
}

fn bench_proxy_to_string_with_auth(c: &mut Criterion) {
    let proxy = Proxy {
        protocol: ProxyType::Http,
        host: "192.168.1.1".to_string(),
        port: 8080,
        username: Some("username".to_string()),
        password: Some("password".to_string()),
        timeout: None,
        exit_ip: None,
    };
    
    c.bench_function("proxy_to_string_with_auth", |b| {
        b.iter(|| {
            let s = black_box(&proxy).to_string(true);
            black_box(s);
        });
    });
}

fn bench_proxy_equality(c: &mut Criterion) {
    let proxy1 = create_test_proxy(1);
    let proxy2 = create_test_proxy(1);
    let proxy3 = create_test_proxy(2);
    
    c.bench_function("proxy_equality_same", |b| {
        b.iter(|| {
            let result = black_box(&proxy1) == black_box(&proxy2);
            black_box(result);
        });
    });
    
    c.bench_function("proxy_equality_different", |b| {
        b.iter(|| {
            let result = black_box(&proxy1) == black_box(&proxy3);
            black_box(result);
        });
    });
}

fn bench_proxy_hashing(c: &mut Criterion) {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    
    let proxy = create_test_proxy(1);
    
    c.bench_function("proxy_hash", |b| {
        b.iter(|| {
            let mut hasher = DefaultHasher::new();
            black_box(&proxy).hash(&mut hasher);
            let hash = hasher.finish();
            black_box(hash);
        });
    });
}

fn bench_proxy_deduplication(c: &mut Criterion) {
    let mut group = c.benchmark_group("proxy_deduplication");
    
    for count in [100, 1000, 10000].iter() {
        let proxies: Vec<Proxy> = (0..*count)
            .map(|i| create_test_proxy(i % 100)) // Create duplicates
            .collect();
        
        group.bench_with_input(BenchmarkId::from_parameter(count), &proxies, |b, proxies| {
            b.iter(|| {
                let unique: HashSet<_> = proxies.iter().cloned().collect();
                black_box(unique);
            });
        });
    }
    
    group.finish();
}

fn bench_proxy_sorting(c: &mut Criterion) {
    use std::time::Duration;
    
    let mut group = c.benchmark_group("proxy_sorting");
    
    for count in [100, 1000, 10000].iter() {
        let mut proxies: Vec<Proxy> = (0..*count)
            .map(|i| {
                let mut p = create_test_proxy(i);
                p.timeout = Some(Duration::from_millis((i % 1000) as u64));
                p
            })
            .collect();
        
        group.bench_with_input(BenchmarkId::from_parameter(count), &proxies, |b, _| {
            b.iter(|| {
                let mut p = proxies.clone();
                p.sort_unstable_by(|a, b| {
                    a.timeout.unwrap_or(Duration::MAX).cmp(&b.timeout.unwrap_or(Duration::MAX))
                });
                black_box(p);
            });
        });
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_proxy_to_string,
    bench_proxy_to_string_with_auth,
    bench_proxy_equality,
    bench_proxy_hashing,
    bench_proxy_deduplication,
    bench_proxy_sorting,
);
criterion_main!(benches);
