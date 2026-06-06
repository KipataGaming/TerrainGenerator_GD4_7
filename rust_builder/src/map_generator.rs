use godot::prelude::*;
use godot::classes::{Image, Object}; 
use godot::classes::image::Format;
use serde::Deserialize;
use image::GenericImageView;

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

    // --- New Networked Search ---
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

    // --- New Networked Fetch ---
    #[var]
    #[export]
    zoom: i32,

    #[var]
    #[export]
    tiles_wide: i32,

    #[var(set = trigger_download)]
    #[export]
    run_download: bool,

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
            base 
        }
    }
}

#[godot_api]
impl MapGenerator {
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

        godot_print!("RUST SAYS: Downloading {}x{} terrain tiles at Zoom {}...", n, n, z);

        // Calculate tile coordinates for the center
        let lat_rad = (lat as f64).to_radians();
        let n_pow = 2.0f64.powi(z);
        let center_x = ((lon + 180.0) / 360.0 * n_pow).floor() as i32;
        let center_y = ((1.0 - (lat_rad.tan() + 1.0/lat_rad.cos()).ln() / std::f64::consts::PI) / 2.0 * n_pow).floor() as i32;

        let start_x = center_x - (n / 2);
        let start_y = center_y - (n / 2);

        let mut global_heights = vec![vec![0.0f32; (n * 256) as usize]; (n * 256) as usize];

        for ty in 0..n {
            for tx in 0..n {
                let x = start_x + tx;
                let y = start_y + ty;
                let url = format!("https://s3.amazonaws.com/elevation-tiles-prod/terrarium/{}/{}/{}.png", z, x, y);

                godot_print!("Fetching tile {}/{}...", (ty * n + tx) + 1, n * n);

                match reqwest::blocking::get(&url) {
                    Ok(response) => {
                        if let Ok(bytes) = response.bytes() {
                            if let Ok(img) = image::load_from_memory(&bytes) {
                                for py in 0..256 {
                                    for px in 0..256 {
                                        let pixel = img.get_pixel(px, py);
                                        let r = pixel[0] as f32;
                                        let g = pixel[1] as f32;
                                        let b = pixel[2] as f32;

                                        // Terrarium formula: (R * 256 + G + B / 256) - 32768
                                        let h = (r * 256.0 + g + b / 256.0) - 32768.0;
                                        
                                        let gy = (ty * 256 + py as i32) as usize;
                                        let gx = (tx * 256 + px as i32) as usize;
                                        global_heights[gy][gx] = h;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => godot_print!("ERROR: Failed to download tile {},{}: {}", x, y, e),
                }
            }
        }

        self.apply_to_terrain(global_heights);
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

        self.apply_to_terrain(heights);
    }

    fn apply_to_terrain(&self, heights: Vec<Vec<f32>>) {
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
                let slope = (dx * dx + dy * dy).sqrt();

                let base_texture_id: u32 = if slope > 1.5 { 1 } else { 0 };
                let control_int: u32 = base_texture_id & 0x1F; 
                let control_float = f32::from_bits(control_int);

                height_bytes.extend_from_slice(&current_height.to_le_bytes());
                control_bytes.extend_from_slice(&control_float.to_le_bytes());
                color_bytes.extend_from_slice(&[255, 255, 255, 255]);
            }
        }

        let gd_height_bytes = PackedByteArray::from(height_bytes.as_slice());
        let gd_control_bytes = PackedByteArray::from(control_bytes.as_slice());
        let gd_color_bytes = PackedByteArray::from(color_bytes.as_slice());

        let height_opt = Image::create_from_data(width as i32, length as i32, false, Format::RF, &gd_height_bytes);
        let control_opt = Image::create_from_data(width as i32, length as i32, false, Format::RF, &gd_control_bytes);
        let color_opt = Image::create_from_data(width as i32, length as i32, false, Format::RGBA8, &gd_color_bytes);

        if let (Some(h_img), Some(ctrl_img), Some(col_img)) = (height_opt, control_opt, color_opt) {
            if let Some(mut terrain_node) = self.base().get_node_or_null(&NodePath::from("../Terrain3D")) {
                let mut storage_var = terrain_node.get(&StringName::from("data"));
                if storage_var.is_nil() {
                    storage_var = terrain_node.get(&StringName::from("storage"));
                }

                if let Ok(mut storage_obj) = storage_var.try_to::<Gd<Object>>() {
                    let mut img_array = Array::<Gd<Image>>::new();
                    img_array.push(&h_img);      
                    img_array.push(&ctrl_img);   
                    img_array.push(&col_img);    
                    
                    storage_obj.call(
                        &StringName::from("import_images"),
                        &[
                            img_array.to_variant(),                  
                            Vector3::new(0.0, 0.0, 0.0).to_variant(), 
                            0.0.to_variant(),                        
                            1.0.to_variant()                         
                        ]
                    );
                    godot_print!("SUCCESS: Terrain painted and updated live!");
                }
            }
            h_img.save_exr(&GString::from("res://rust_heightmap.exr"));
        }
    }
}