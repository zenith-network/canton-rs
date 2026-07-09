use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};

use daml_lf::dar_manifest::DarManifest;

const EXAMPLE_MANIFEST_STR: &str = include_str!("assets/example-manifest.in");

fn bench_manifest_parse(c: &mut Criterion) {
    c.bench_function("manifest_parse", |b| {
        b.iter(|| {
            let _ = black_box(DarManifest::parse(EXAMPLE_MANIFEST_STR));
        })
    });
}

criterion_group!(benches, bench_manifest_parse,);
criterion_main!(benches);
