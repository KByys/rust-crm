pub struct Table;

// 时间类型固定 25 个字符
// 手机号码 15个字符

impl Table {
    /// administrator -1 总经理，0 管理员， 1 成员
    pub const USER_TABLE: &str = "CREATE TABLE IF NOT EXISTS user(
            id VARCHAR(15) NOT NULL,
            name VARCHAR(20) NOT NULL,
            password BINARY(16) NOT NULL,
            department VARCHAR(30) NULL,
            permissions INT NOT NULL,
            identity INT NOT NULL,
            sex INT NOT NULL,
            PRIMARY KEY (id),
            FOREIGN KEY (department) REFERENCES department(value)
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
            post VARCHAR(15),
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
    /// 签到表
    /// 
    /// file 为附件的base64编码地址，如果有多个附件则用`&`隔开
    pub const SING_TABLE: &str = "CREATE TABLE IF NOT EXISTS sign(
            signer VARCHAR(15) NOT NULL,
            customer VARCHAR(15),
            address VARCHAR(150),
            sign_time VARCHAR(25),
            file TEXT
        )
    ";
    
    // TODO 还需要修改
    /// 预约拜访时间表
    /// status, 0 未完成， 1 已完成, 2 逾期（当天没有完成即逾期）
    /// applicant, 发起者，可以是自己，也可以是上司
    /// salesman，如果客户不为空，则只能是该客户的业务员
    /// appointment，联系时间，格式，YYYY-MM-DD HH:MM
    pub const APPOINTMENT_TABLE: &str = "CREATE TABLE IF NOT EXISTS appointment(
            applicant VARCHAR(15) NOT NULL,
            salesman VARCHAR(15) NOT NULL,
            customer VARCHAR(15) NOT NULL,
            appointment VARCHAR(16) NOT NULL,
            finish_time VARCHAR(16),
            status INT NOT NULL,
            theme VARCHAR(30),
            content TEXT
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
