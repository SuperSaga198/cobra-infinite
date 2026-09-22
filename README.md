# Cobra

Cobra is an experimental programming language and compiler platform written in
Rust. It is designed as a long-lived foundation for the larger Cobra Infinite
blueprint: a multi-target compiler, portable runtime, standard library, and
verification pipeline.

## First release

The first usable slice is intentionally small and real:

- hand-written lexer with strings, numbers, comments, keywords, and operators
- recursive-descent parser with a stable AST
- human-readable source diagnostics
- tree-walking interpreter with lexical scope and user functions
- `print` and `type_of` built-ins
- CLI commands for running, checking, inspecting tokens, and printing the AST

## Run it

```sh
cargo run -p cobrac -- run examples/hello.cbr
cargo run -p cobrac -- check examples/hello.cbr
cargo run -p cobrac -- tokens examples/hello.cbr
cargo test --workspace
```

The language uses `.cbr` files:

```cobra
fn greet(name) {
    return "hello, " + name;
}

print(greet("world"));
```

## Repository map

- `constellation-core/compiler/` — lexer, parser, and diagnostics crates
- `quantum-runtimes/runtime_core_os/` — portable interpreter runtime
- `interstellar-suites/cobrac/` — command-line tool
- `automated-labs/` — reserved for verifier and fuzzing harnesses
- `.cosmic/` — planetary node and timing configuration
- `.infra/` — future distributed build configuration

Cargo is the source of truth for the local compiler. Bazel files document the
future distributed build boundary and stay intentionally lightweight until the
compiler has multiple production targets.