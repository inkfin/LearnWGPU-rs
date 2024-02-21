# Some implementation of CGT-521 code using Rust+WGPU

```shell
cargo run --release -p <path>
```

[install wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
```shell
wasm-pack build <path> --target web

RUSTFLAGS=--cfg=web_sys_unstable_apis wasm-pack build --target web
```
