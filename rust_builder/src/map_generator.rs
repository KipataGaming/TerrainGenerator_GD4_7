use godot::prelude::*;
use godot::classes::{Image, Object, Engine}; 
use godot::classes::image::Format;
use serde::Deserialize;
use image::GenericImageView;
use rand::{Rng, RngExt};

#[derive(Deserialize, Debug)]
struct PhotonResult {
    features: Vec<PhotonFeature>,
}

#[derive(Deserialize, Debug)]
struct PhotonFeature {
    geometry: PhotonGeometry,
    properties: PhotonProperties,
}

#[derive(Deserialize, Debug)]
struct PhotonGeometry {
    coordinates: Vec<f64>, // [lon, lat]
}

#[derive(Deserialize, Debug)]
struct PhotonProperties {
    name: Option<String>,
    country: Option<String>,
    state: Option<String>,
}

#[derive(GodotClass)]
#[class(tool, base=Node)]
pub struct MapGenerator {
    #[var(set = trigger_generation)]
    #[export]
    generate_now: bool,

    #[var]
    #[export]
    png_path: GString,

    #[var]
    #[export]
    max_height: f32,

    // --- Networked Search ---
    #[var]
    #[export]
    search_query: GString,

    #[var(set = trigger_search)]
    #[export]
    run_search: bool,

    #[var]
    #[export]
    latitude: f64,

    #[var]
    #[export]
    longitude: f64,

    // --- Networked Fetch ---
    #[var]
    #[export]
    zoom: i32,

    #[var]
    #[export]
    tiles_wide: i32,

    #[var(set = trigger_download)]
    #[export]
    run_download: bool,

    // --- Enhancements ---
    #[var]
    #[export]
    enable_post_processing: bool,

    #[var]
    #[export]
    smoothing_iterations: i32,

    #[var]
    #[export]
    erosion_iterations: i32,

    #[var]
    #[export]
    erosion_amount: f32,

    #[var]
    #[export]
    island_mode: bool,

    #[var]
    #[export]
    map_km_wide: f32,

    #[var]
    #[export]
    recommended_spacing: f32,

    #[var]
    #[export]
    altitude_offset: f32,

    base: Base<Node>,
}

#[godot_api]
impl INode for MapGenerator {
    fn init(base: Base<Node>) -> Self {
        Self { 
            generate_now: false,
            png_path: GString::from("res://heightmap.png"),
            max_height: 400.0,
            search_query: GString::from("Mount Everest"),
            run_search: false,
            latitude: 27.9881,
            longitude: 86.9250,
            zoom: 12,
            tiles_wide: 1,
            run_download: false,
            enable_post_processing: true,
            smoothing_iterations: 2,
            erosion_iterations: 50000,
            erosion_amount: 0.1,
            island_mode: false,
            map_km_wide: 0.0,
            recommended_spacing: 1.0,
            altitude_offset: 0.0,
            base 
        }
    }

    fn process(&mut self, _delta: f64) {
        if Engine::singleton().is_editor_hint() {
            // Update accuracy feedback in inspector
            let meters_per_pixel = self.calculate_meters_per_pixel();
            self.map_km_wide = (self.tiles_wide as f32 * 256.0 * meters_per_pixel as f32) / 1000.0;
            self.recommended_spacing = meters_per_pixel as f32;
        }
    }
}

#[godot_api]
impl MapGenerator {
    fn calculate_meters_per_pixel(&self) -> f64 {
        let lat_rad = (self.latitude as f64).to_radians();
        // Standard Earth circumference / (2^zoom * 256 pixels per tile)
        (40075016.686 * lat_rad.cos()) / (2.0f64.powi(self.zoom as i32) * 256.0)
    }

    #[func]
    fn trigger_search(&mut self, value: bool) {
        if !value { return; }
        self.run_search = false;

        let query = self.search_query.to_string();
        godot_print!("RUST SAYS: Searching for '{}'...", query);

        let url = format!("https://photon.komoot.io/api/?q={}&limit=1", query);
        
        match reqwest::blocking::get(&url) {
            Ok(response) => {
                if let Ok(result) = response.json::<PhotonResult>() {
                    if let Some(feature) = result.features.first() {
                        self.longitude = feature.geometry.coordinates[0];
                        self.latitude = feature.geometry.coordinates[1];
                        
                        let name = feature.properties.name.clone().unwrap_or_default();
                        let state = feature.properties.state.clone().unwrap_or_default();
                        let country = feature.properties.country.clone().unwrap_or_default();
                        
                        godot_print!("SUCCESS: Found {} ({}, {}) in {}, {}", name, self.latitude, self.longitude, state, country);
                    } else {
                        godot_print!("WARNING: No locations found for '{}'", query);
                    }
                } else {
                    godot_print!("ERROR: Could not parse search results.");
                }
            }
            Err(e) => godot_print!("ERROR: Search request failed: {}", e),
        }
    }

    #[func]
    fn trigger_download(&mut self, value: bool) {
        if !value { return; }
        self.run_download = false;

        let lat = self.latitude;
        let lon = self.longitude;
        let z = self.zoom;
        let n = self.tiles_wide;
        let offset_y = self.altitude_offset;
        let spacing = self.calculate_meters_per_pixel() as f32;
        
        let lat_rad = (lat as f64).to_radians();
        let n_pow = 2.0f64.powi(z);
        
        // Exact fractional tile coordinates
        let exact_tile_x = (lon + 180.0) / 360.0 * n_pow;
        let exact_tile_y = (1.0 - (lat_rad.tan() + 1.0/lat_rad.cos()).ln() / std::f64::consts::PI) / 2.0 * n_pow;

        let center_x = exact_tile_x.floor() as i32;
        let center_y = exact_tile_y.floor() as i32;
        
        let start_x = center_x - (n / 2);
        let start_y = center_y - (n / 2);

        // Calculate world offset so search location is at Godot origin
        let sub_tile_x = (exact_tile_x - start_x as f64) * 256.0;
        let sub_tile_y = (exact_tile_y - start_y as f64) * 256.0;
        let world_offset_x = -(sub_tile_x as f32 * spacing);
        let world_offset_z = -(sub_tile_y as f32 * spacing);

        let terrain_node_path = NodePath::from("../Terrain3D");
        let terrain_node_opt = self.base().get_node_or_null(&terrain_node_path);
        
        if terrain_node_opt.is_none() {
            godot_print!("ERROR: Could not find sibling node 'Terrain3D'.");
            return;
        }

        let terrain_node = terrain_node_opt.unwrap();
        let mut storage_var = terrain_node.get(&StringName::from("data"));
        if storage_var.is_nil() {
            storage_var = terrain_node.get(&StringName::from("storage"));
        }

        if storage_var.is_nil() {
            godot_print!("ERROR: Could not access Terrain3D storage.");
            return;
        }
        
        let storage_obj = storage_var.try_to::<Gd<Object>>().unwrap();

        godot_print!("RUST SAYS: Starting synchronous download of {}x{} tiles...", n, n);

        use std::time::Duration;
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("TerrainSynthesizer-Godot/1.0")
            .build()
            .unwrap();

        let size = (n * 256) as usize;
        let mut heights = vec![vec![0.0f32; size]; size];

        for i in 0..(n*n) {
            let tx = i % n;
            let ty = i / n;
            let x = start_x + tx;
            let y = start_y + ty;
            let url = format!("https://s3.amazonaws.com/elevation-tiles-prod/terrarium/{}/{}/{}.png", z, x, y);

            match client.get(&url).send() {
                Ok(response) => {
                    if response.status().is_success() {
                        if let Ok(bytes) = response.bytes() {
                            if let Ok(img) = image::load_from_memory(&bytes) {
                                for py in 0..256 {
                                    for px in 0..256 {
                                        let pixel = img.get_pixel(px, py);
                                        let h = (pixel[0] as f32 * 256.0 + pixel[1] as f32 + pixel[2] as f32 / 256.0) - 32768.0;
                                        let gy = (ty * 256 + py as i32) as usize;
                                        let gx = (tx * 256 + px as i32) as usize;
                                        heights[gy][gx] = h;
                                    }
                                }
                            }
                        }
                    }
                }
                Err(_) => {}
            }
            if (i + 1) % 4 == 0 || (i + 1) == n * n {
                godot_print!("PROGRESS: Downloaded {}/{} tiles...", i + 1, n * n);
            }
        }

        if self.enable_post_processing {
            godot_print!("RUST SAYS: Post-processing started...");
            
            if self.island_mode {
                godot_print!(" - Applying Island Taper...");
                self.apply_island_mode(&mut heights, size);
            }

            if self.smoothing_iterations > 0 {
                godot_print!(" - Smoothing ({} passes)...", self.smoothing_iterations);
                for _ in 0..self.smoothing_iterations {
                    heights = self.apply_smoothing(&heights, size);
                }
            }

            if self.erosion_iterations > 0 {
                godot_print!(" - Hydraulic Erosion ({} droplets)...", self.erosion_iterations);
                self.apply_erosion(&mut heights, size, spacing);
            }
        }

        godot_print!("RUST SAYS: Processing finished. Applying to Terrain3D with offset: ({}, {}, {})", world_offset_x, offset_y, world_offset_z);
        Self::static_apply_to_terrain(heights, storage_obj, Vector3::new(world_offset_x, offset_y, world_offset_z), spacing);
    }

    fn apply_island_mode(&self, heights: &mut Vec<Vec<f32>>, size: usize) {
        let center = size as f32 / 2.0;
        let max_dist = center * 0.85; 

        for y in 0..size {
            for x in 0..size {
                let dx = x as f32 - center;
                let dy = y as f32 - center;
                let dist = (dx * dx + dy * dy).sqrt();
                
                if dist > max_dist {
                    let t = (dist - max_dist) / (center - max_dist);
                    let weight = (1.0 - t.clamp(0.0, 1.0)).powf(3.0); 
                    heights[y][x] *= weight;
                }
            }
        }
    }

    fn apply_smoothing(&self, heights: &Vec<Vec<f32>>, size: usize) -> Vec<Vec<f32>> {
        let mut new_heights = heights.clone();
        for y in 1..size-1 {
            for x in 1..size-1 {
                let mut sum = 0.0;
                for ny in (y-1)..=(y+1) {
                    for nx in (x-1)..=(x+1) {
                        sum += heights[ny][nx];
                    }
                }
                new_heights[y][x] = sum / 9.0;
            }
        }
        new_heights
    }

    fn apply_erosion(&self, heights: &mut Vec<Vec<f32>>, size: usize, spacing: f32) {
        let mut rng = rand::rng();
        
        let inertia: f32 = 0.05;
        let sediment_capacity_factor: f32 = 4.0;
        let min_sediment_capacity: f32 = 0.01;
        let dissolve_speed: f32 = self.erosion_amount * 0.1;
        let deposit_speed: f32 = self.erosion_amount * 0.1;
        let evaporate_speed: f32 = 0.01;
        let gravity: f32 = 4.0;
        let max_droplet_lifetime: i32 = 30;

        for _ in 0..self.erosion_iterations {
            let mut pos_x: f32 = rng.random_range(0.0..(size as f32 - 1.0));
            let mut pos_y: f32 = rng.random_range(0.0..(size as f32 - 1.0));
            let mut dir_x: f32 = 0.0;
            let mut dir_y: f32 = 0.0;
            let mut vel: f32 = 1.0;
            let mut water: f32 = 1.0;
            let mut sediment: f32 = 0.0;

            for _ in 0..max_droplet_lifetime {
                let node_x = pos_x as usize;
                let node_y = pos_y as usize;
                let x_offset = pos_x - node_x as f32;
                let y_offset = pos_y - node_y as f32;

                let h00 = heights[node_y][node_x];
                let h10 = heights[node_y][node_x + 1];
                let h01 = heights[node_y + 1][node_x];
                let h11 = heights[node_y + 1][node_x + 1];

                let grad_x = ((h10 - h00) * (1.0 - y_offset) + (h11 - h01) * y_offset) / spacing;
                let grad_y = ((h01 - h00) * (1.0 - x_offset) + (h11 - h10) * x_offset) / spacing;

                dir_x = dir_x * inertia - grad_x * (1.0 - inertia);
                dir_y = dir_y * inertia - grad_y * (1.0 - inertia);

                let len = (dir_x * dir_x + dir_y * dir_y).sqrt();
                if len != 0.0 {
                    dir_x /= len;
                    dir_y /= len;
                }

                pos_x += dir_x;
                pos_y += dir_y;

                if pos_x < 0.0 || pos_x >= (size as f32 - 1.0) || pos_y < 0.0 || pos_y >= (size as f32 - 1.0) {
                    break;
                }

                let new_height = heights[pos_y as usize][pos_x as usize];
                let delta_height = new_height - h00;

                let sediment_capacity = ((-delta_height).max(min_sediment_capacity) * vel * water * sediment_capacity_factor).max(min_sediment_capacity);

                if sediment > sediment_capacity || delta_height > 0.0 {
                    let amount_to_deposit = if delta_height > 0.0 { delta_height.min(sediment) } else { (sediment - sediment_capacity) * deposit_speed };
                    sediment -= amount_to_deposit;
                    heights[node_y][node_x] += amount_to_deposit * (1.0 - x_offset) * (1.0 - y_offset);
                    heights[node_y][node_x + 1] += amount_to_deposit * x_offset * (1.0 - y_offset);
                    heights[node_y + 1][node_x] += amount_to_deposit * (1.0 - x_offset) * y_offset;
                    heights[node_y + 1][node_x + 1] += amount_to_deposit * x_offset * y_offset;
                } else {
                    let amount_to_erode = ((sediment_capacity - sediment) * dissolve_speed).min(-delta_height);
                    sediment += amount_to_erode;
                    heights[node_y][node_x] -= amount_to_erode * (1.0 - x_offset) * (1.0 - y_offset);
                    heights[node_y][node_x + 1] -= amount_to_erode * x_offset * (1.0 - y_offset);
                    heights[node_y + 1][node_x] -= amount_to_erode * (1.0 - x_offset) * y_offset;
                    heights[node_y + 1][node_x + 1] -= amount_to_erode * x_offset * y_offset;
                }

                vel = (vel * vel + delta_height * gravity).abs().sqrt();
                water *= 1.0 - evaporate_speed;
            }
        }
    }

    #[func]
    fn trigger_generation(&mut self, value: bool) {
        if !value { return; }
        self.generate_now = false;
        
        let source_image_opt = Image::load_from_file(&self.png_path);
        if source_image_opt.is_none() {
            godot_print!("ERROR: Could not load '{}'", self.png_path);
            return;
        }
        
        let source_image = source_image_opt.unwrap();
        let width = source_image.get_width() as usize;
        let length = source_image.get_height() as usize;
        
        let mut heights = vec![vec![0.0f32; width]; length];
        for y in 0..length {
            for x in 0..width {
                let pixel_color = source_image.get_pixel(x as i32, y as i32);
                heights[y][x] = pixel_color.r * self.max_height;
            }
        }

        if let Some(terrain_node) = self.base().get_node_or_null(&NodePath::from("../Terrain3D")) {
            let mut storage_var = terrain_node.get(&StringName::from("data"));
            if storage_var.is_nil() {
                storage_var = terrain_node.get(&StringName::from("storage"));
            }
            if let Ok(storage_obj) = storage_var.try_to::<Gd<Object>>() {
                Self::static_apply_to_terrain(heights, storage_obj, Vector3::new(0.0, self.altitude_offset, 0.0), 1.0);
            }
        }
    }

    fn static_apply_to_terrain(heights: Vec<Vec<f32>>, mut storage_obj: Gd<Object>, offset: Vector3, spacing: f32) {
        let length = heights.len();
        let width = heights[0].len();
        
        let mut height_bytes = Vec::with_capacity(width * length * 4);
        let mut control_bytes = Vec::with_capacity(width * length * 4);
        let mut color_bytes = Vec::with_capacity(width * length * 4); 

        for y in 0..length {
            for x in 0..width {
                let current_height = heights[y][x];
                
                let nx = if x + 1 < width { x + 1 } else { x };
                let ny = if y + 1 < length { y + 1 } else { y };
                
                let dx = heights[y][nx] - current_height;
                let dy = heights[ny][x] - current_height;
                let slope = (dx * dx + dy * dy).sqrt() / spacing;

                let base_texture_id: u32 = if slope > 1.0 { 
                    0 // ROCK
                } else { 
                    1 // GRASS
                };

                let control_int: u32 = (base_texture_id & 0x1F) << 27; 
                let control_float = f32::from_bits(control_int);

                // Particles (Alpha) only on Grass (ID 1) AND above 10m water line
                let particle_mask: u8 = if current_height > 10.0 && base_texture_id == 1 { 255 } else { 0 };

                height_bytes.extend_from_slice(&current_height.to_le_bytes());
                control_bytes.extend_from_slice(&control_float.to_le_bytes());
                color_bytes.extend_from_slice(&[255, 255, 255, particle_mask]);
            }
        }

        let gd_height_bytes = PackedByteArray::from(height_bytes.as_slice());
        let gd_control_bytes = PackedByteArray::from(control_bytes.as_slice());
        let gd_color_bytes = PackedByteArray::from(color_bytes.as_slice());

        let height_opt = Image::create_from_data(width as i32, length as i32, false, Format::RF, &gd_height_bytes);
        let control_opt = Image::create_from_data(width as i32, length as i32, false, Format::RF, &gd_control_bytes);
        let color_opt = Image::create_from_data(width as i32, length as i32, false, Format::RGBA8, &gd_color_bytes);

        if let (Some(mut h_img), Some(ctrl_img), Some(col_img)) = (height_opt, control_opt, color_opt) {
            let mut img_array = Array::<Gd<Image>>::new();
            img_array.push(&h_img);      
            img_array.push(&ctrl_img);   
            img_array.push(&col_img);    
            
            storage_obj.call_deferred(
                &StringName::from("import_images"),
                &[
                    img_array.to_variant(),                  
                    offset.to_variant(), 
                    0.0.to_variant(),                        
                    1.0.to_variant()                         
                ]
            );
            
            h_img.call_deferred(&StringName::from("save_exr"), &[GString::from("res://rust_heightmap.exr").to_variant()]);
        }
    }
}