pub mod auth_service;
pub mod config_service;
pub mod question_service;
pub mod types;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_services_module_exists() {
        // このテストは services モジュールの存在を確認する
        // types モジュールが利用可能であることを確認
        let _types_module = std::any::type_name::<types::AuthStatus>();
        assert!(_types_module.contains("AuthStatus"));
    }
}