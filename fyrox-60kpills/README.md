# Fyrox 60k Pill Benchmark

Benchmark scene matching `pill_demo`, for comparison with Pill Engine and Bevy.

## Usage

Build:
```bash
cargo build --release
```

Run (`assets/` must be alongside the binary or under this directory):
```bash
cargo run --release
# or:
./target/release/fyrox_demo
```

Set env vars as needed:
- `FYROX_PILL_COUNT=0` — no pills
- `FYROX_PILL_COUNT=1` — single pill
- `FYROX_PILL_COUNT=60000` — full benchmark
- `FYROX_SPAWN_BATCH=2000` — batch size for spawning (reduce stutter)

Example:
```bash
FYROX_PILL_COUNT=60000 FYROX_SPAWN_BATCH=2000 ./target/release/fyrox_demo
```

## Assets

- `assets/textures/pill_color.png` — albedo
- `assets/textures/pill_normal.png` — normal map
- `assets/models/pill.obj` — reference mesh (not loaded; cylinders generated procedurally)

## Links

- [Fyrox Engine](https://github.com/FyroxEngine/Fyrox)
- [Fyrox Book](https://fyrox-book.github.io/)
