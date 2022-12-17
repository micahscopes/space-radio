# Space Radio
This is a CLAP/VST3 plugin to send audio effect parameters out into other spaces via OSC.

## Usage

For now, the OSC address is hardcoded to `127.0.0.1:9009`.
### Build
After installing Rust run:
```
cargo xtask bundle gain --release
```

## Thanks
This plugin was made possible by the amazing [NIH-plug](https://github.com/robbert-vdh/nih-plug) tooling from [@robbert-vdh](https://github.com/robbert-vdh/)