extends MeshInstance3D

@export var water_level: float = 0.0 : set = set_water_level

func _ready():
    set_water_level(water_level)

func set_water_level(value: float):
    water_level = value
    global_position.y = water_level
