-- 下拉框选项
CREATE TABLE IF NOT EXISTS drop_down_box (
    -- 下拉框名称，如 department
    name VARCHAR(30) NOT NULL,
    value VARCHAR(30) NOT NULL,
    create_time VARCHAR(25) NOT NULL,
    PRIMARY KEY (name, value)
);

INSERT
    IGNORE INTO drop_down_box (name, value, create_time)
VALUES
    ('payment', '现金', '0000-00-00 00:00:00'),
    ('payment', '银行转账', '0000-00-00 00:00:01'),
    ('payment', '对公转账', '0000-00-00 00:00:02');

INSERT
    IGNORE INTO drop_down_box (name, value, create_time)
VALUES
    ('department', '总经办', '0000-00-00 00:00:00');

INSERT
    IGNORE INTO drop_down_box (name, value, create_time)
VALUES
    ('storehouse', '主仓库', '0000-00-00 00:00:00');

CREATE TABLE IF NOT EXISTS custom_fields (
    -- 0 客户字段， 1 产品字段
    ty INT NOT NULL,
    -- 0 文本字段，1 时间字段，2下拉框字段
    display VARCHAR(2) NOT NULL,
    -- 字段显示文本
    value VARCHAR(30) NOT NULL,
    create_time VARCHAR(25) NOT NULL,
    PRIMARY KEY (ty, display, value)
);

CREATE TABLE IF NOT EXISTS custom_field_data (
    -- 0 客户字段， 1 产品字段
    fields INT NOT NULL,
    -- 0 文本字段，1 时间字段，2下拉框字段
    ty INT NOT NULL,
    -- 客户或产品对应的id
    id VARCHAR(150) NOT NULL,
    -- 字段显示文本
    display VARCHAR(30) NOT NULL,
    -- 对应的数据
    value VARCHAR(30) NOT NULL,
    -- create_time VARCHAR(25) NOT NULL,
    PRIMARY KEY (fields, ty, display, id)
);

-- 自定义字段下拉选项
CREATE TABLE IF NOT EXISTS custom_field_option (
    -- 0 客户字段， 1 产品字段
    ty INT NOT NULL,
    -- 显示的文本
    display VARCHAR(30) NOT NULL,
    -- 选项值
    value VARCHAR(30) NOT NULL,
    create_time VARCHAR(25) NOT NULL,
    PRIMARY KEY (ty, display, value)
);

-- 角色表
CREATE TABLE IF NOT EXISTS roles (
    id VARCHAR(50) NOT NULL,
    name VARCHAR(50) NOT NULL,
    PRIMARY KEY (id)
);

INSERT
    IGNORE INTO roles (id, name)
VALUES
    ('root', '总经理'),
    ('admin', '管理员'),
    ('salesman', '销售员');

-- 用户表
CREATE TABLE IF NOT EXISTS user(
    id VARCHAR(150) NOT NULL,
    smartphone VARCHAR(15) NOT NULL UNIQUE,
    name VARCHAR(20) NOT NULL,
    password BINARY(16) NOT NULL,
    department VARCHAR(30) NOT NULL,
    role VARCHAR(50) NOT NULL,
    sex INT NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (role) REFERENCES roles(id)
);

-- 离职员工表
CREATE TABLE IF NOT EXISTS leaver (
    id VARCHAR(150) NOT NULL,
    PRIMARY KEY (id)
);

-- 客户表
CREATE TABLE IF NOT EXISTS customer (
    id VARCHAR(150) NOT NULL,
    smartphone VARCHAR(15) NOT NULL UNIQUE,
    name VARCHAR(50) NOT NULL,
    company VARCHAR(50) NOT NULL,
    is_share INT NOT NULL,
    sex INT NOT NULL,
    chat VARCHAR(50) NOT NULL,
    need TEXT NOT NULL,
    fax VARCHAR(50) NOT NULL,
    post VARCHAR(50) NOT NULL,
    industry VARCHAR(30) NOT NULL,
    birthday VARCHAR(10) NOT NULL,
    level VARCHAR(30) NOT NULL,
    create_time VARCHAR(25) NOT NULL,
    address VARCHAR(150) NOT NULL,
    -- 备注
    remark TEXT NOT NULL,
    -- 跟踪状态
    status VARCHAR(30),
    -- 来源
    source TEXT,
    -- 职务
    role VARCHAR(30),
    -- 客户类型
    ty VARCHAR(30),
    -- 客户标签
    tag VARCHAR(30),
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS extra_customer_data (
    id VARCHAR(150) NOT NULL,
    salesman VARCHAR(150) NULL,
    added_date VARCHAR(25) NULL,
    -- 上传拜访时间, 暂时的值，后面需要用联合查询替换
    -- last_visited_time VARCHAR(25) NULL,
    -- 已拜访次数
    -- visited_count INT NOT NULL,
    -- 上次成交时间 暂时的值，后面需要用联合查询替换
    last_transaction_time VARCHAR(25) NULL,
    push_to_sea_date VARCHAR(25) NULL,
    pop_from_sea_date VARCHAR(25) NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (id) REFERENCES customer(id),
    FOREIGN KEY (salesman) REFERENCES user(id)
);

CREATE TABLE IF NOT EXISTS customer_colleague(
    id VARCHAR(150) NOT NULL,
    customer VARCHAR(150) NOT NULL,
    phone VARCHAR(15) NOT NULL,
    name VARCHAR(10) NOT NULL,
    create_time VARCHAR(25),
    PRIMARY KEY(id),
    FOREIGN KEY (customer) REFERENCES customer(id)
);

CREATE TABLE IF NOT EXISTS sign(
    id VARCHAR(150) NOT NULL,
    signer VARCHAR(150) NOT NULL,
    customer VARCHAR(150),
    address VARCHAR(150),
    location VARCHAR(25),
    sign_time VARCHAR(25),
    file TEXT,
    content TEXT,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS appointment(
    id VARCHAR(150) NOT NULL,
    applicant VARCHAR(150) NOT NULL,
    salesman VARCHAR(150) NULL,
    customer VARCHAR(150) NULL,
    appointment VARCHAR(25) NOT NULL,
    finish_time VARCHAR(25),
    theme VARCHAR(30),
    content TEXT,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS appoint_comment (
    id VARCHAR(150) NOT NULL,
    applicant VARCHAR(150) NOT NULL,
    appoint VARCHAR(150) NOT NULL,
    create_time VARCHAR(25) NOT NULL,
    comment TEXT,
    PRIMARY KEY (id)
);


CREATE TABLE IF NOT EXISTS token(
    ty INT NOT NULL,
    id VARCHAR(150) NOT NULL,
    tbn BIGINT NULL,
    PRIMARY KEY(ty, id)
);

-- 产品表
-- num 编号
-- cover 封面的地址
CREATE TABLE IF NOT EXISTS product(
    id VARCHAR(150) NOT NULL,
    num VARCHAR(50) NOT NULL,
    name VARCHAR(50) NOT NULL,
    specification VARCHAR(10) NOT NULL,
    cover VARCHAR(150) NULL,
    model VARCHAR(20) NOT NULL,
    unit VARCHAR(30) NOT NULL,
    amount INT NOT NULL,
    product_type VARCHAR(30) NOT NULL,
    price FLOAT NOT NULL,
    create_time VARCHAR(25) NOT NULL,
    barcode VARCHAR(20) NOT NULL,
    explanation TEXT,
    storehouse VARCHAR(30) NOT NULL,
    PRIMARY KEY (id)
);

-- 产品编号，用于记录顺序
CREATE TABLE IF NOT EXISTS product_num(
    name VARCHAR(100) NOT NULL,
    num INT NOT NULL,
    PRIMARY KEY (name)
);

-- 报告表
CREATE TABLE IF NOT EXISTS report(
    id VARCHAR (150) NOT NULL,
    applicant VARCHAR(150) NOT NULL,
    reviewer VARCHAR(150) NOT NULL,
    -- 0 日报，1 周报，2 月报
    ty INT NOT NULL,
    create_time VARCHAR (25) NOT NULL,
    -- 关联客户
    ac VARCHAR(150) NULL,
    contents TEXT NOT NULL,
    send_time VARCHAR (25) NULL,
    processing_time VARCHAR(25) NULL,
    opinion TEXT NULL,
    -- 0 审批通过，1 不通过, 2未审批，
    status INT NOT NULL,
    PRIMARY KEY (id)
);
-- 报告抄送人
CREATE TABLE IF NOT EXISTS report_cc (
    cc VARCHAR(150) NOT NULL,
    report VARCHAR(150) NOT NULL,
    PRIMARY KEY (cc, report)
);


-- 报告回复
CREATE TABLE IF NOT EXISTS report_reply(
    id VARCHAR(150) NOT NULL,
    report VARCHAR(150) NOT NULL,
    create_time VARCHAR(25) NOT NULL,
    contents TEXT NOT NULL,
    applicant VARCHAR(150) NOT NULL,
    PRIMARY KEY (id)
);