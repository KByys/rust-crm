use mysql::{prelude::Queryable, PooledConn};

use crate::database::get_conn;

pub static mut ROLE_TABLES: RoleTable = RoleTable::empty();
#[derive(Debug)]
pub struct RoleTable {
    table: [(String, String); 20],
    pos: usize,
}

impl RoleTable {
    pub const fn empty() -> RoleTable {
        RoleTable {
            table: [
                (String::new(), String::new()),
                (String::new(), String::new()),
                (String::new(), String::new()),
                (String::new(), String::new()),
                (String::new(), String::new()),
                (String::new(), String::new()),
                (String::new(), String::new()),
                (String::new(), String::new()),
                (String::new(), String::new()),
                (String::new(), String::new()),
                (String::new(), String::new()),
                (String::new(), String::new()),
                (String::new(), String::new()),
                (String::new(), String::new()),
                (String::new(), String::new()),
                (String::new(), String::new()),
                (String::new(), String::new()),
                (String::new(), String::new()),
                (String::new(), String::new()),
                (String::new(), String::new()),
            ],
            pos: 0,
        }
    }
    pub fn init(&mut self) {
        let mut conn = get_conn().expect("初始化角色表时连接数据库失败");
        let map: Vec<(String, String)> = conn
            .query_map("SELECT id, name FROM roles", |(id, name)| (id, name))
            .expect("初始化角色表时查询失败");
        for (i, (id, name)) in map.into_iter().enumerate() {
            self.table[i] = (id, name);
            self.pos += 1;
        }
    }
    pub fn update(&mut self, conn: &mut PooledConn) -> mysql::Result<()> {
        let map: Vec<(String, String)> =
            conn.query_map("SELECT id, name FROM roles", |(id, name)| (name, id))?;
        self.table = TABLE.clone();
        self.pos = 0;
        for (i, (id, name)) in map.into_iter().enumerate() {
            self.table[i] = (id, name);
            self.pos += 1;
        }
        Ok(())
    }
    pub fn get_name(&self, id: &str) -> Option<&str> {
        for (id_k, name) in &self.table[..self.pos] {
            if id_k == id {
                return Some(name);
            }
        }
        None
    }
    pub fn get_name_uncheck(&self, id: &str) -> String {
        self.get_name(id).unwrap().to_owned()
    }
    pub fn get_id(&self, name: &str) -> Option<&str> {
        for (id, name_k) in &self.table[..self.pos] {
            if name == name_k {
                return Some(id);
            }
        }
        None
    }
}
static TABLE: [(String, String); 20] = [
    (String::new(), String::new()),
    (String::new(), String::new()),
    (String::new(), String::new()),
    (String::new(), String::new()),
    (String::new(), String::new()),
    (String::new(), String::new()),
    (String::new(), String::new()),
    (String::new(), String::new()),
    (String::new(), String::new()),
    (String::new(), String::new()),
    (String::new(), String::new()),
    (String::new(), String::new()),
    (String::new(), String::new()),
    (String::new(), String::new()),
    (String::new(), String::new()),
    (String::new(), String::new()),
    (String::new(), String::new()),
    (String::new(), String::new()),
    (String::new(), String::new()),
    (String::new(), String::new()),
];
