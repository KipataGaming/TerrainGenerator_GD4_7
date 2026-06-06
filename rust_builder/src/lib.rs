use godot::prelude::*;

mod map_generator;

struct TerrainBuilderExtension;

#[gdextension]
unsafe impl ExtensionLibrary for TerrainBuilderExtension {}
