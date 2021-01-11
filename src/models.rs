use crate::schema::{groups, users};

#[derive(Identifiable, Queryable, Associations, PartialEq, Debug)]
#[table_name = "groups"]
pub struct Group {
    pub id: i32,
    pub name: String,
    pub api_id: String,
}


#[derive(Identifiable, Queryable, PartialEq, Debug)]
#[table_name = "users"]
pub struct User {
    pub id: i32,
    pub tg_id: i32,
    pub tg_name: String,
    pub notify: bool,
    pub group_id: i32,
}
