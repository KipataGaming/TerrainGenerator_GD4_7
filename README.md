# TerrainSynthesizer

A high-performance procedural terrain generation tool for Godot 4.x, built with **Rust** using the **GDExtension (gdext)** pipeline. This project focuses on direct memory injection into **Terrain3D** for efficient, live editor-based terrain sculpting and painting.

## Features

- **Rust-Powered Processing:** Fast conversion of real-world DEM (Digital Elevation Model) PNGs into 3D terrain data.
- **Terrain3D Integration:** Uses direct C++ memory injection via `import_images` for live updates in the Godot editor.
- **Slope-Based Painting:** Automatically calculates terrain slopes to apply textures (e.g., Grass vs. Rock) during generation.
- **EXR Export:** Automatically saves processed heightmaps to OpenEXR format for further refinement.

## Technical Context

- **Engine:** Godot 4.7 **beta 4** (Note: Beta 5 has just released; this project is currently verified for Beta 4).
- **Language:** Rust (Stable).
- **Plugin Dependencies:** [Terrain3D](https://github.com/TokisanGames/Terrain3D).

## Usage

1. **Setup:** Ensure you have the `Terrain3D` plugin enabled in your Godot project.
2. **Node Setup:** Add a `MapGenerator` node (the Rust class) as a sibling to a `Terrain3D` node in your scene.
3. **Configuration:**
   - `png_path`: The path to your grayscale heightmap (e.g., `res://heightmap.png`).
   - `max_height`: The maximum 3D height scale for pure white pixels.
4. **Generate:** Toggle the `Generate Now` checkbox in the inspector to trigger the generation process.

## Building from Source

To compile the Rust GDExtension:

1. Navigate to the `rust_builder` directory.
2. Run `cargo build`.
3. The resulting binary will be placed in `godot_project/bin/`.

## License

MIT (See LICENSE file for details).
