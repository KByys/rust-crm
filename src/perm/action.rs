use std::collections::HashMap;
lazy_static::lazy_static! {
    pub static ref PERMISSION_GROUPS: HashMap<&'static str, Vec<&'static str>> = {
        groups()
    };
}

fn groups() -> HashMap<&'static str, Vec<&'static str>> {
    [
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
    .into_iter()
    .collect()
}

#[forbid(unused)]
pub static ROLE: [&str; 4] = [
    RoleGroup::CREATE,
    RoleGroup::UPDATE,
    RoleGroup::DELETE,
    RoleGroup::CHANGE_ROLE,
];

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
pub static DEPARTMENT: [&str; 4] = [
    DepartmentGroup::CREATE,
    DepartmentGroup::UPDATE,
    DepartmentGroup::DELETE,
    DepartmentGroup::DELETE_ROLE,
];

pub struct DepartmentGroup;

impl DepartmentGroup {
    pub const CREATE: &str = "create";
    pub const UPDATE: &str = "update";
    pub const DELETE: &str = "delete";
    pub const DELETE_ROLE: &str = "delete_role";
}

#[forbid(unused)]
pub static CUSTOMER: [&str; 10] = [
    CustomerGroup::ACTIVATION,
    CustomerGroup::QUERY,
    CustomerGroup::ENTER_CUSTOMER_DATA,
    CustomerGroup::UPDATE_CUSTOMER_DATA,
    CustomerGroup::DELETE_CUSTOMER_DATA,
    CustomerGroup::QUERY_PUB_SEA,
    CustomerGroup::TRANSFER_CUSTOMER,
    CustomerGroup::EXPORT_DATA,
    CustomerGroup::RELEASE_CUSTOMER,
    CustomerGroup::ADD_APPOINT,
];

pub struct CustomerGroup;
impl CustomerGroup {
    pub const ACTIVATION: &str = "activation";
    pub const QUERY: &str = "query";
    pub const ENTER_CUSTOMER_DATA: &str = "enter_customer_data";
    pub const UPDATE_CUSTOMER_DATA: &str = "update_customer_data";
    pub const DELETE_CUSTOMER_DATA: &str = "delete_customer_data";
    /// 查看不属于任何部门的公海客户信息
    pub const QUERY_PUB_SEA: &str = "query_pub_sea";
    pub const TRANSFER_CUSTOMER: &str = "transfer_customer";
    pub const EXPORT_DATA: &str = "export_data";
    pub const RELEASE_CUSTOMER: &str = "release_customer";
    pub const ADD_APPOINT: &str = "add_appoint";
}
#[forbid(unused)]
pub static STOREHOUSE: [&str; 2] = [StorehouseGroup::ACTIVATION, StorehouseGroup::PRODUCT];

pub struct StorehouseGroup;

impl StorehouseGroup {
    pub const ACTIVATION: &str = "activation";
    pub const PRODUCT: &str = "product";
    // TODO:
}

#[forbid(unused)]
pub static PURCHASE: [&str; 2] = [PurchaseGroup::ACTIVATION, PurchaseGroup::QUERY];
pub struct PurchaseGroup;

impl PurchaseGroup {
    pub const ACTIVATION: &str = "activation";
    pub const QUERY: &str = "query";
}

#[forbid(unused)]
pub static FINANCE: [&str; 2] = [FinanceGroup::ACTIVATION, FinanceGroup::QUERY];
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
pub static FORM: [&str; 1] = [FormGroup::QUERT];
pub struct FormGroup;

impl FormGroup {
    pub const QUERT: &str = "query";
}

#[forbid(unused)]
pub static OTHER_GROUP: [&str; 5] = [
    OtherGroup::QUERY_SIGN_IN,
    OtherGroup::CUSTOM_FIELD,
    OtherGroup::DROP_DOWN_BOX,
    OtherGroup::SEA_RULE,
    OtherGroup::COMPANY_STAFF_DATA,
];
pub struct OtherGroup;

impl OtherGroup {
    pub const QUERY_SIGN_IN: &str = "query_sign_in";
    pub const CUSTOM_FIELD: &str = "custom_field";
    pub const DROP_DOWN_BOX: &str = "drop_down_box";
    pub const SEA_RULE: &str = "sea_rule";
    pub const COMPANY_STAFF_DATA: &str = "company_staff_data";
}
