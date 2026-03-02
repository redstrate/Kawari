# Profiling

Install a better `addr2line` that doesn't take forever to load and works better:
```shell
cargo install  addr2line --features="bin"
```

Depending on your system you need to force frame pointers otherwise the callgraphs are wrong:

```shell
RUSTFLAGS="-Cforce-frame-pointers=yes" cargo build --workspace --features oodle
```

And now you finally run `perf`:

```shell
PATH=$HOME/.cargo/bin:$PATH LD_LIBRARY_PATH=oodle/ perf record -g --aio --call-graph dwarf ./target/debug/kawari-world
```
