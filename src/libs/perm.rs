use mysql::{prelude::Queryable, PooledConn};

use crate::Response;

pub enum Identity {
    Boss,
    /// 管理员(权限组，部门)
    Administrator(usize, String),
    /// 员工(权限组，部门)
    Staff(usize, String),
}

impl Identity {
    pub fn new(id: &str, conn: &mut PooledConn) -> Result<Self, Response> {
        let data: Option<(usize, Option<String>, Option<usize>)> = conn.query_first(format!(
            "SELECT identity, department, permissions FROM user WHERE id = '{}'",
            id
        ))?;
        if let Some((identity, department, permissions)) = data {
            let ident = match identity {
                0 => Self::Boss,
                1 => Self::Administrator(permissions.unwrap(), department.unwrap()),
                2 => Self::Staff(permissions.unwrap(), department.unwrap()),
                _ => {
                    return Err(Response::invalid_value(format!(
                        "identity的值非法，为{}",
                        identity
                    )))
                }
            };
            Ok(ident)
        } else {
            Err(Response::not_exist(format!("用户 {} 不存在", id)))
        }
    }
}
