use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use proxy_spider::parsers::PROXY_REGEX;

fn bench_parse_simple_proxy(c: &mut Criterion) {
    let text = "http://192.168.1.1:8080";
    
    c.bench_function("parse_simple_proxy", |b| {
        b.iter(|| {
            let captures: Vec<_> = PROXY_REGEX.captures_iter(black_box(text)).collect();
            black_box(captures);
        });
    });
}

fn bench_parse_proxy_with_auth(c: &mut Criterion) {
    let text = "http://user:password@192.168.1.1:8080";
    
    c.bench_function("parse_proxy_with_auth", |b| {
        b.iter(|| {
            let captures: Vec<_> = PROXY_REGEX.captures_iter(black_box(text)).collect();
            black_box(captures);
        });
    });
}

fn bench_parse_multiple_proxies(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_multiple_proxies");
    
    for count in [10, 100, 1000, 10000].iter() {
        let proxies: Vec<String> = (0..*count)
            .map(|i| format!("http://192.168.{}.{}:8080", i / 256, i % 256))
            .collect();
        let text = proxies.join("\n");
        
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &text, |b, text| {
            b.iter(|| {
                let captures: Vec<_> = PROXY_REGEX.captures_iter(black_box(text)).collect();
                black_box(captures);
            });
        });
    }
    
    group.finish();
}

fn bench_parse_mixed_protocols(c: &mut Criterion) {
    let text = r#"
        http://192.168.1.1:8080
        https://192.168.1.2:8443
        socks4://192.168.1.3:1080
        socks5://user:pass@192.168.1.4:1080
        192.168.1.5:3128
    "#;
    
    c.bench_function("parse_mixed_protocols", |b| {
        b.iter(|| {
            let captures: Vec<_> = PROXY_REGEX.captures_iter(black_box(text)).collect();
            black_box(captures);
        });
    });
}

fn bench_parse_from_large_text(c: &mut Criterion) {
    // Simulate parsing proxies from a large HTML page
    let mut text = String::from("<html><body>");
    for i in 0..1000 {
        text.push_str(&format!(
            "<p>Some random text here http://192.168.{}.{}:8080 more text</p>",
            i / 256,
            i % 256
        ));
    }
    text.push_str("</body></html>");
    
    c.bench_function("parse_from_large_text", |b| {
        b.iter(|| {
            let captures: Vec<_> = PROXY_REGEX.captures_iter(black_box(&text)).collect();
            black_box(captures);
        });
    });
}

fn bench_parse_with_domains(c: &mut Criterion) {
    let text = r#"
        http://proxy1.example.com:8080
        http://proxy2.example.com:8080
        http://proxy3.example.com:8080
        http://proxy4.example.com:8080
        http://proxy5.example.com:8080
    "#;
    
    c.bench_function("parse_with_domains", |b| {
        b.iter(|| {
            let captures: Vec<_> = PROXY_REGEX.captures_iter(black_box(text)).collect();
            black_box(captures);
        });
    });
}

criterion_group!(
    benches,
    bench_parse_simple_proxy,
    bench_parse_proxy_with_auth,
    bench_parse_multiple_proxies,
    bench_parse_mixed_protocols,
    bench_parse_from_large_text,
    bench_parse_with_domains,
);
criterion_main!(benches);
