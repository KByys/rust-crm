use std::collections::HashMap;

lazy_static::lazy_static! {
    pub static ref PERMISSION_GROUPS: HashMap<&'static str, Vec<&'static str>> = {
        let mut map = HashMap::new();
        for (k, v) in groups() {
            map.insert(k, v);
        }
        map
    };
}

fn groups() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        ("role", ROLE.to_vec()),
        ("account", ACCOUNT.to_vec()),
        ("customer", CUSTOMER.to_vec()),
        ("storehouse", STOREHOUSE.to_vec()),
        ("finance", FINANCE.to_vec()),
        ("purchase", PURCHASE.to_vec()),
        ("department", DEPARTMENT.to_vec()),
        ("approval", APPROVAL.to_vec()),
        ("form", FORM.to_vec()),
        ("other", OTHER_GROUP.to_vec()),
    ]
}

#[forbid(unused)]
pub static ROLE: [&str; 4] = ["create", "update", "delete", "change_role"];

pub struct RoleGroup;

impl RoleGroup {
    pub const CREATE: &str = "create";
    pub const UPDATE: &str = "update";
    pub const DELETE: &str = "delete";
    /// 角色调动
    pub const CHANGE_ROLE: &str = "change_role";
}

#[forbid(unused)]
pub static ACCOUNT: [&str; 2] = ["create", "update"];

pub struct AccountGroup;
impl AccountGroup {
    pub const CREATE: &str = "create";
    pub const DELETE: &str = "delete";
}

#[forbid(unused)]
pub static DEPARTMENT: [&str; 4] = ["create", "update", "delete", "delete_role"];

pub struct DepartmentGroup;

impl DepartmentGroup {
    pub const CREATE: &str = "create";
    pub const UPDATE: &str = "update";
    pub const DELETE: &str = "delete";
    pub const DELETE_ROLE: &str = "delete_role";
}

#[forbid(unused)]
pub static CUSTOMER: [&str; 9] = [
    CustomerGroup::ACTIVATION,
    "query",
    "enter_cutomer_data",
    "update_cutomer_data",
    CustomerGroup::DELETE_CUSTOMER_DATA,
    "query_pub_sea",
    "transfer_customer",
    "export_data",
    "releases_customer",
];

pub struct CustomerGroup;
impl CustomerGroup {
    pub const ACTIVATION: &str = "activation";
    pub const QUERY: &str = "query";
    pub const ENTER_CUSTOMER_DATA: &str = "enter_cutomer_data";
    pub const UPDATE: &str = "update_customer_data";
    pub const DELETE_CUSTOMER_DATA: &str = "delete_customer_data";
    /// 查看不属于任何部门的公海客户信息
    pub const QUERY_PUB_SEA: &str = "query_pub_sea";
    pub const TRANSFER_CUSTOMER: &str = "transfer_customer";
    pub const EXPORT_DATA: &str = "export_data";
    pub const RELEASE_CUSTOMER: &str = "release_customer";
}
#[forbid(unused)]
pub static STOREHOUSE: [&str; 3] = ["activation", "query_product", "prodcut"];

pub struct StorehouseGroup;

impl StorehouseGroup {
    pub const ACTIVATION: &str = "activation";
    pub const QUERY_PRODUCT: &str = "query_product";
    pub const PRODUCT: &str = "product";
    // TODO:
}

#[forbid(unused)]
pub static PURCHASE: [&str; 2] = ["activation", "query"];
pub struct PurchaseGroup;

impl PurchaseGroup {
    pub const ACTIVATION: &str = "activation";
    pub const QUERY: &str = "query";
}

#[forbid(unused)]
pub static FINANCE: [&str; 2] = ["activation", "query"];
pub struct FinanceGroup;

impl FinanceGroup {
    pub const ACTIVATION: &str = "activation";
    pub const QUERY: &str = "query";
}

#[forbid(unused)]
pub static APPROVAL: [&str; 2] = ["query_approval", "receive_approval"];
pub struct ApprovalGroup;
impl ApprovalGroup {
    pub const QUERY_APPROVAL: &str = "query_approval";
    pub const RECEIVE_APPROVAL: &str = "receive_approval";
}

#[forbid(unused)]
pub static FORM: [&str; 1] = ["query"];
pub struct FormGroup;

impl FormGroup {
    pub const QUERT: &str = "query";
}

#[forbid(unused)]
pub static OTHER_GROUP: [&str; 5] = [
    OtherGroup::QUERT_CHECK_IN,
    OtherGroup::CUSTOM_FIELD,
    OtherGroup::DROP_DOWN_BOX,
    OtherGroup::SEA_RULE,
    OtherGroup::COMPANY_STAFF_DATA,
];
pub struct OtherGroup;

impl OtherGroup {
    pub const QUERT_CHECK_IN: &str = "query_check_in";
    pub const CUSTOM_FIELD: &str = "custom_field";
    pub const DROP_DOWN_BOX: &str = "drop_down_box";
    pub const SEA_RULE: &str = "sea_rule";
    pub const COMPANY_STAFF_DATA: &str = "company_staff_data";
}
