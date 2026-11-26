use criterion::{Criterion, criterion_group, criterion_main};
use oxc::{
    allocator::Allocator,
    parser::{ParseOptions, Parser},
    span::SourceType,
};

fn pipeline_surface(c: &mut Criterion) {
    let js_source = r#"
function boot() {
  const message = "hello";
  const answer = 40 + 2;
  return { message, answer };
}
window.runtime = boot();
"#;
    let liveview_source = include_str!("../assets/liveview_client.js");
    let embeddable_source = r#"
import type { Foo } from "./foo";
export type { Foo };
export default function boot() {
  return 42;
}
"#;
    let js_source_type = SourceType::from_path("runtime.js").expect("static js path should parse");

    c.bench_function("parse_only_js", |b| {
        b.iter(|| {
            let allocator = Allocator::default();
            let parser_return = Parser::new(&allocator, js_source, js_source_type)
                .with_options(ParseOptions {
                    parse_regular_expression: true,
                    ..ParseOptions::default()
                })
                .parse();
            assert!(parser_return.errors.is_empty(), "parse should succeed");
        });
    });

    c.bench_function("analyze_source_js", |b| {
        b.iter(|| {
            let _ = rs_js::minify("runtime.js", js_source)
                .javascript()
                .compatible_csp()
                .analyze()
                .report()
                .expect("analyze should succeed");
        });
    });

    c.bench_function("analyze_liveview_client_asset", |b| {
        b.iter(|| {
            let _ = rs_js::minify("liveview_client.js", liveview_source)
                .javascript()
                .analyze()
                .report()
                .expect("liveview analyze should succeed");
        });
    });

    c.bench_function("minify_source_js", |b| {
        b.iter(|| {
            let _ = rs_js::minify("runtime.js", js_source)
                .javascript()
                .run()
                .expect("minify should succeed");
        });
    });

    c.bench_function("minify_liveview_client_asset", |b| {
        b.iter(|| {
            let _ = rs_js::minify("liveview_client.js", liveview_source)
                .javascript()
                .run()
                .expect("liveview minify should succeed");
        });
    });

    c.bench_function("obfuscate_source_js", |b| {
        b.iter(|| {
            let _ = rs_js::minify("runtime.js", js_source)
                .javascript()
                .compatible_csp()
                .obfuscate()
                .run()
                .expect("obfuscate should succeed");
        });
    });

    c.bench_function("normalize_embeddable_ts", |b| {
        b.iter(|| {
            let _ = rs_js::normalize_embeddable("runtime.ts", embeddable_source)
                .expect("normalize should succeed");
        });
    });

    c.bench_function("minify_embeddable_ts", |b| {
        b.iter(|| {
            let _ = rs_js::minify("runtime.ts", embeddable_source)
                .typescript()
                .embeddable()
                .run()
                .expect("embeddable minify should succeed");
        });
    });

    c.bench_function("minify_source_js_repeated_16", |b| {
        b.iter(|| {
            for _ in 0..16 {
                let _ = rs_js::minify("runtime.js", js_source)
                    .javascript()
                    .run()
                    .expect("minify should succeed");
            }
        });
    });
}

criterion_group!(benches, pipeline_surface);
criterion_main!(benches);
