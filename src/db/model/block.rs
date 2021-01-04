use diesel::{Insertable, Queryable};

// #[derive(Queryable, Insertable)]
pub struct Block {
    pub id: i32,
    pub title: String,
    pub body: String,
    pub published: bool,
}