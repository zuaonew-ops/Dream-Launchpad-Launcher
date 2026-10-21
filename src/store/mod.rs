//! 本地数据：数据目录布局、每版本设置、账号与登录。

pub mod accounts;
pub mod auth;
pub mod auth_msa;
pub mod paths;
pub mod settings;

pub use paths::Paths;
