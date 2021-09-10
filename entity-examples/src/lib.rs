#![allow(dead_code)]

use xiayu::prelude::*;

#[derive(Entity)]
pub struct User {
    pub id: i32,
}

#[derive(Entity)]
pub struct Post {
    pub id: i32,
    pub user_id: i32,
}

#[derive(Entity)]
pub struct Recipe {
    pub name: String,
    pub ingredients: String,
}

#[derive(Entity)]
pub struct Cat {
    pub master_id: i32,
    pub ingredients: String,
}

#[derive(Entity)]
pub struct Dog {
    pub slave_id: i32,
    pub age: i32,
    pub ingredients: String,
}

#[derive(Debug, Entity)]
pub struct Bar {
    pub id: i32,
    pub uniq_val: i32,
}
