@tool
extends MeshInstance3D

@export var water_level: float = 0.0 : set = set_water_level
@export var wave_speed: float = 2.0
@export var wave_height: float = 0.5

var time_passed: float = 0.0

func _ready():
    set_water_level(water_level)

func _process(delta):
    # Calculate wave offset regardless of editor/runtime
    time_passed += delta
    var wave_offset = sin(time_passed * wave_speed) * wave_height
    
    # In editor, we want to see the base level. In game, we want the animation.
    if Engine.is_editor_hint():
        position.y = water_level
    else:
        position.y = water_level + wave_offset

func set_water_level(value: float):
    water_level = value
    if is_inside_tree():
        position.y = water_level
    else:
        call_deferred("set_water_level", value)
