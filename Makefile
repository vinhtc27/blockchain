info:
	dust && tokei

unuse:
	cargo +nightly udeps --all-targets

run:
	cargo run

watch:
	Bcargo watch -x "run"

release:
	cargo run --release

update:
	rustup update && cargo outdated && cargo update && cargo upgrade && cargo audit

clean:
	cargo sweep --time 7 && cargo sweep --toolchains nightly-aarch64-apple-darwin && cargo sweep --installed