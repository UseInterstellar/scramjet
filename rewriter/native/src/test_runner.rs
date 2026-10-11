#[cfg(test)]
mod test {
	use std::fs;

	use crate::{RewriterOptions, rewriter::NativeRewriter};
	use boa_engine::{
		Context, NativeFunction, Source, js_str, js_string,
		object::ObjectInitializer,
		property::{Attribute, PropertyDescriptorBuilder},
	};

	fn create_context() -> Context {
		let mut context = Context::default();

		let window = ObjectInitializer::new(&mut context).build();
		context
			.register_global_property(js_str!("window"), window, Attribute::READONLY)
			.unwrap();
		let top = ObjectInitializer::new(&mut context).build();
		context
			.register_global_property(js_str!("top"), top, Attribute::READONLY)
			.unwrap();
		context
			.global_object()
			.define_property_or_throw(
				js_str!("location"),
				PropertyDescriptorBuilder::new()
					.get(
						NativeFunction::from_copy_closure(|_, _, _| Ok(js_str!("location").into()))
							.to_js_function(context.realm()),
					)
					.set(
						NativeFunction::from_copy_closure(|_, _, _| {
							panic!("fail: window.location got set")
						})
						.to_js_function(context.realm()),
					)
					.build(),
				&mut context,
			)
			.unwrap();

		context
			.register_global_callable(
				js_string!("fail"),
				0,
				NativeFunction::from_copy_closure(|_, _, _| {
					panic!("fail");
				}),
			)
			.unwrap();

		context
			.eval(Source::from_bytes(
				br#"
const self = window;
function $wrap(val) {
	if (val === window || val == "location" || val == top) return "";

    return val;
}

function $prop(prop) {
    if (prop === "location" || prop === "top" || prop === "eval") {
        return "$sj_"+prop
    }
    return prop
}

const $gwrap = $wrap;

function $scramitize(val) { return val }

function assert(val) {
	if (!val) fail();
}

function $tryset(target, op, value) {
    if (target == "location") return true;
    return false;
}
function $clean() {

}

function check(val) {
    if (val === window || val === top || val === "location") fail();
}
			    "#,
			))
			.unwrap();

		context
	}

	#[test]
	fn google() {
		let source_text = include_str!("../sample/google.js");

		let rewriter = NativeRewriter::default();
		rewriter.rewrite_default(source_text).unwrap();
	}

	#[test]
	fn rewrite_tests() {
		let files = fs::read_dir("./tests").unwrap();

		let opts = crate::RewriterOptions {
			prefix: String::from("/scrammedjet/"),
			wrapfn: String::from("$wrap"),
			wrappropertybase: String::from("$sj_"),
			wrappropertyfn: String::from("$prop"),
			cleanrestfn: String::from("$clean"),
			importfn: String::from("$import"),
			rewritefn: String::from("$rewrite"),
			metafn: String::from("$meta"),
			setrealmfn: String::from("$setrealm"),
			pushsourcemapfn: String::from("$pushsourcemap"),
			trysetfn: String::from("$tryset"),
			templocid: String::from("$temploc"),
			tempunusedid: String::from("$tempunused"),
			base: String::from("https://google.com/glorngle/si.js"),
			sourcetag: String::from("glongle1"),
			is_module: false,
			capture_errors: false,
			do_sourcemaps: false,
			scramitize: false,
			strict_rewrites: true,
			destructure_rewrites: true,
		};

		let mut rewriter = NativeRewriter::new(&opts);

		for file in files.map(|x| x.unwrap()) {
			if !file.path().extension().unwrap().eq_ignore_ascii_case("js") {
				continue;
			}

			let content = fs::read_to_string(file.path()).unwrap();

			let rewritten = rewriter.rewrite(&content, &opts).unwrap();
			println!("{}", std::str::from_utf8(&rewritten.js).unwrap());

			let mut ctx = create_context();

			ctx.eval(Source::from_bytes(rewritten.js.as_slice()))
				.unwrap();

			println!("PASS");

			rewriter.reset();
		}
	}

	#[test]
	fn loop_bodies_rewrite_imports() {
		// import() and import.meta inside for...of / for...in bodies used to stay raw, which resolved
		// against the real module url (broke Vite style preload helpers)
		let source = r#"
for (const dep of deps) { import("./a.js"); void import.meta.url; }
for (const k in deps) { import("./b.js"); void import.meta.url; }
async function f() { for await (const d of deps) { await import("./c.js"); void import.meta.url; } }
for (const a of deps) for (const b of deps) { import("./d.js"); void import.meta.url; }
for (let i = 0; i < 1; i++) { import("./e.js"); void import.meta.url; }
"#;

		for destructure_rewrites in [true, false] {
			let opts = crate::RewriterOptions {
				importfn: String::from("$import"),
				metafn: String::from("$meta"),
				is_module: true,
				destructure_rewrites,
				base: String::from("https://example.com/x/y.js"),
				..crate::RewriterOptions::default()
			};
			let mut rewriter = NativeRewriter::new(&opts);
			let out = rewriter.rewrite(source, &opts).unwrap();
			let js = std::str::from_utf8(&out.js).unwrap();

			// every import( is the argument of $import(, every import.meta is wrapped by $meta(
			assert_eq!(js.matches("import(").count(), js.matches("$import(").count(), "raw import() left: {js}");
			assert_eq!(js.matches("$import(").count(), 5, "{js}");
			assert_eq!(js.matches("$meta(import.meta").count(), 5, "{js}");
			assert_eq!(js.matches("import.meta").count(), 5, "raw import.meta left: {js}");
		}
	}
}
