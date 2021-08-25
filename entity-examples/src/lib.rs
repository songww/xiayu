use xiayu::prelude::*;

#[derive(Entity)]
pub struct User {
    id: i32,
}

#[derive(Entity)]
pub struct Post {
    id: i32,
    user_id: i32,
}

#[derive(Entity)]
pub struct Recipe {
    name: String,
    ingredients: String,
}

#[derive(Entity)]
pub struct Cat {
    master_id: i32,
    ingredients: String,
}

#[derive(Entity)]
pub struct Dog {
    slave_id: i32,
    age: i32,
    ingredients: String,
}
