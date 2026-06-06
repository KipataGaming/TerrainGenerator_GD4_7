use godot::prelude::*;
use godot::classes::{Image, Object}; 
use godot::classes::image::Format;

#[derive(GodotClass)]
#[class(tool, base=Node)]
pub struct MapGenerator {
    #[var(set = trigger_generation)]
    #[export]
    generate_now: bool,

    // Let the user type the exact name of their PNG file!
    #[var]
    #[export]
    png_path: GString,

    // How tall should the pure white pixels be in 3D space?
    #[var]
    #[export]
    max_height: f32,

    base: Base<Node>,
}

#[godot_api]
impl INode for MapGenerator {
    fn init(base: Base<Node>) -> Self {
        Self { 
            generate_now: false,
            png_path: GString::from("res://heightmap.png"),
            max_height: 200.0, // Sierra Nevadas are tall!
            base 
        }
    }
}

#[godot_api]
impl MapGenerator {
    #[func]
    fn trigger_generation(&mut self, _value: bool) {
        godot_print!("========================================");
        godot_print!("RUST SAYS: Reading real-world PNG from {}...", self.png_path);
        
        // ---------------------------------------------------------
        // 1. LOAD THE REAL-WORLD PNG FILE
        // ---------------------------------------------------------
        let source_image_opt = Image::load_from_file(&self.png_path);
        
        if source_image_opt.is_none() {
            godot_print!("ERROR: Could not load '{}'. Did you put it in the godot_project folder?", self.png_path);
            self.generate_now = false;
            return;
        }
        
        let source_image = source_image_opt.unwrap();
        let width = source_image.get_width() as usize;
        let length = source_image.get_height() as usize;
        
        godot_print!("SUCCESS: Loaded {}x{} PNG. Carving mountains...", width, length);

        // ---------------------------------------------------------
        // PASS 1: CONVERT PIXEL COLORS TO 3D HEIGHTS
        // ---------------------------------------------------------
        let mut heights = vec![vec![0.0f32; width]; length];

        for y in 0..length {
            for x in 0..width {
                // Read the pixel color. (Returns a Color object with r, g, b, a from 0.0 to 1.0)
                let pixel_color = source_image.get_pixel(x as i32, y as i32);
                
                // Heightmaps are grayscale, so we just read the Red channel.
                // Pure black (0.0) stays 0. Pure white (1.0) becomes our max_height.
                let height_val = pixel_color.r * self.max_height;
                
                heights[y][x] = height_val;
            }
        }

        // ---------------------------------------------------------
        // PASS 2: CALCULATE SLOPES AND PAINT TEXTURES
        // ---------------------------------------------------------
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

                // TEXTURE LOGIC: Real-world DEMs can have crazy sheer drops.
                // We set the steepness threshold to 1.5 to catch actual cliffs.
                let base_texture_id: u32 = if slope > 1.5 { 
                    1 // ID 1: Rock
                } else { 
                    0 // ID 0: Grass
                };

                let control_int: u32 = base_texture_id & 0x1F; 
                let control_float = f32::from_bits(control_int);

                height_bytes.extend_from_slice(&current_height.to_le_bytes());
                control_bytes.extend_from_slice(&control_float.to_le_bytes());
                color_bytes.extend_from_slice(&[255, 255, 255, 255]);
            }
        }

        godot_print!("RUST SAYS: Math finished! Injecting directly to Terrain3D...");

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
                    godot_print!("SUCCESS: Terrain painted and updated live in the editor!");
                } else {
                    godot_print!("ERROR: Found Terrain3D, but could not access its data resource.");
                }
            } else {
                godot_print!("WARNING: Could not find sibling node named 'Terrain3D'.");
            }
            
            h_img.save_exr(&GString::from("res://rust_heightmap.exr"));
            
        } else {
            godot_print!("ERROR: Failed to create Godot Images.");
        }

        godot_print!("========================================");
        self.generate_now = false;
    }
}