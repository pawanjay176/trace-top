# trace-top

`trace-top` is a lightweight local OpenTelemetry trace receiver and terminal UI for inspecting instrumented programs.

It listens for OTLP/gRPC trace exports, stores recent spans in memory, and shows trace waterfalls and aggregate timing data for local debugging.

## Usage
 
```
cargo run --release -- 0.0.0.0:4317 # Listens on port 4317 for trace exports
```

## Screenshots
