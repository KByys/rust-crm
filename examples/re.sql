
-- 黑名单设置
CREATE TABLE IF NOT EXISTS blacklist(id VARCHAR(15), PRIMARY KEY (id));

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
-- cc 抄送人，ac 管理客户， reviewer 批阅者
-- ty, 0 日报，1 周报，2 月报
-- status, 0 未审批，1 审批通过，2 审批未通过, 3 未发送
-- 不添加外键
CREATE TABLE IF NOT EXISTS report(
    id VARCHAR (150) NOT NULL,
    applicant VARCHAR(15) NOT NULL,
    reviewer VARCHAR(15) NOT NULL,
    ty INT NOT NULL,
    status INT NOT NULL,
    create_time VARCHAR (25) NOT NULL,
    cc VARCHAR(15) NULL,
    ac VARCHAR(15) NULL,
    contents TEXT NOT NULL,
    send_time VARCHAR (25) NULL,
    processing_time VARCHAR(25) NULL,
    opinion TEXT NULL,
    PRIMARY KEY (id)
);

-- 报告回复
CREATE TABLE IF NOT EXISTS report_reply(
    id VARCHAR(55) NOT NULL,
    report_id VARCHAR(55) NOT NULL,
    create_time VARCHAR(25) NOT NULL,
    contents TEXT NOT NULL,
    respondent VARCHAR(15) NOT NULL,
    PRIMARY KEY (id)
);