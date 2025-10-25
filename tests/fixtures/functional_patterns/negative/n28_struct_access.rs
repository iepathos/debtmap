struct Point { x: i32, y: i32 }

fn distance_squared(p: Point) -> i32 {
    p.x * p.x + p.y * p.y
}
