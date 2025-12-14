use diesel::prelude::*;

#[derive(Insertable, Queryable, Selectable)]
#[diesel(table_name = crate::schema::service_account)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(belongs_to(User))]
pub struct ServiceAccount {
    pub id: i64,
    pub user_id: i64,
    pub max_ex: i32,
}

#[derive(Insertable, Queryable, Selectable)]
#[diesel(table_name = crate::schema::user)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password: String,
}

#[derive(Insertable, Queryable, Selectable)]
#[diesel(table_name = crate::schema::session)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(belongs_to(User))]
pub struct Session {
    pub user_id: i64,
    pub time: String,
    pub service: String,
    pub sid: String,
}

#[declare_sql_function]
extern "SQL" {
    fn datetime() -> diesel::sql_types::Text;
}
