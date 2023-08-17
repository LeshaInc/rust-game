$env:BEVY_ASSET_ROOT=$PSScriptRoot
$env:WGPU_BACKEND="vulkan"
cargo r --features bevy/dynamic_linking,bevy/trace_tracy