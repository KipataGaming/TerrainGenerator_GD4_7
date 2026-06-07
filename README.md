# TerrainSynthesizer

A high-performance procedural terrain generation tool for Godot 4.x, built with **Rust** using the **GDExtension (gdext)** pipeline. This project focuses on direct memory injection into **Terrain3D** for efficient, live editor-based terrain sculpting and painting.

## Features

- **Rust-Powered Processing:** Fast conversion of real-world DEM data into 3D terrain.
- **Advanced Post-Processing:**
    - **Hydraulic Erosion:** Simulates natural water flow to carve realistic valleys and ridges.
    - **Smoothing:** Multi-pass blur to remove digital artifacts.
    - **Island Mode:** Tapers terrain edges into the sea for clean, island-style generation.
- **Depth-Aware Water:** Includes an Ocean Controller and shader for depth-blending, foam generation, and altitude synchronization.
- **Spatial Accuracy:** Real-world scale calculation (km-wide) and precise centering of locations to the Godot world origin.
- **Terrain3D Integration:** Direct memory injection for live editor-based updates.

## Technical Context

- **Engine:** Godot 4.7 **beta 4**.
- **Language:** Rust (Stable).
- **Plugin Dependencies:** [Terrain3D](https://github.com/TokisanGames/Terrain3D).

## Usage

1. **Setup:** Ensure `Terrain3D` is enabled. Add a `MapGenerator` node as a sibling to `Terrain3D`.
2. **Search/Download:** Enter a query in `Search Query` and toggle `Run Search` to locate a position, then use `Run Download` to fetch real-world terrain.
3. **Configuration:**
   - **Scale:** Copy the `Recommended Spacing` value into the `Terrain3D` node's `Vertex Spacing`.
   - **Post-Processing:** Use the erosion and smoothing sliders to refine the look.
   - **Ocean:** Add an Ocean Mesh with `ocean.gdshader` and link it to the `Water Node Path` for automatic altitude sync.

## Building from Source

1. Navigate to `rust_builder`.
2. Run `cargo build`.
3. The binary in `godot_project/bin/` will update.

## License

MIT.
