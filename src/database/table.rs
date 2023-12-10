pub struct Table;

// 时间类型固定 25 个字符
// 手机号码 15个字符


impl Table {
    /// administrator -1 总经理，0 管理员， 1 成员
    pub const USER_TABLE: &str = "CREATE TABLE IF NOT EXISTS user(
            id VARCHAR(15) NOT NULL,
            name VARCHAR(20) NOT NULL,
            password BINARY(16) NOT NULL,
            department VARCHAR(15) NULL,
            permissions INT NOT NULL,
            administrator INT NOT NULL,
            sex INT NOT NULL,
            PRIMARY KEY (id)
        )
    ";
    /// 客户数据
    pub const CUSTOMER_TABLE: &str = "CREATE TABLE IF NOT EXISTS customer(
            id VARCHAR(15) NOT NULL,
            name VARCHAR(20) NOT NULL,
            company VARCHAR(40) NOT NULL,
            is_share INT NOT NULL,
            sex INT NOT NULL,
            salesman VARCHAR(15) NOT NULL,
            chat VARCHAR(30),
            next_visit_time VARCHAR(25),
            need TEXT,
            fax VARCHAR(15),
            address VARCHAR(100),
            industry VARCHAR(30) NULL,
            birthday VARCHAR(8),
            remark TEXT NULL,
            create_time VARCHAR(25),
            ty VARCHAR(30),
            tag VARCHAR(30),
            status VARCHAR(30),
            source VARCHAR(30),
            role VARCHAR(30),
            PRIMARY KEY (id),
            FOREIGN KEY (salesman) REFERENCES user(id)
        )
    ";
    /// 客户登录信息
    pub const CUSTOMER_LOGIN_TABLE: &str = "CREATE TABLE IF NOT EXISTS customer_login(
            id VARCHAR(15) NOT NULL,
            password BINARY(16) NOT NULL,
            PRIMARY KEY (id),
            FOREIGN KEY (id) REFERENCES customer(id)
        )
    ";

    /// 拜访信息记录表
    /// 
    /// appendix 存放附件地址的base64编码，如果包含多个附件地址，用 `&`隔开
    pub const VISITED_RECORD_TABLE: &str = "CREATE TABLE IF NOT EXISTS visited_record(
             user_id VARCHAR(15) NOT NULL,
             customer_id VARCHAR(15) NOT NULL,
             visited_time VARCHAR(25) NOT NULL,
             address VARCHAR(100) NOT NULL,
             appendix VARCHAR TEXT
        )
    ";
    /// 预约拜访时间表
    pub const APPOINTMENT_TABLE: &str = "CREATE TABLE IF NOT EXISTS appointment(
            user_id VARCHAR(15) NOT NULL,
            customer_id VARCHAR(15) NOT NULL,
            appointment VARCHAR(25) NOT NULL,
        )
    ";
    /// ty, 0 公司人员， 1 客户
    ///
    /// tbn 签发时间如果在此时间前则不可用，用于确保用户修改密码后让之前分发的所有token失效
    pub const TOKEN: &str = "CREATE TABLE IF NOT EXISTS token(
            ty INT NOT NULL,
            id VARCHAR(15) NOT NULL,
            tbn BIGINT NULL,
            PRIMARY KEY(ty, id)
        )
    ";
}
