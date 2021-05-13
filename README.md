# LBM win Unity

Build the project:

```
cargo build && cp ./target/debug/lbmaf.dll ../Assets/Plugins/lbmaf.dll && rm ../Assets/Plugins/lbmaf.dll.meta
```

This will copy the output to the Unity's Plugin folder for Native Plugin Rendering interface.