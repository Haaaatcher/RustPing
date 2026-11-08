//! TOML 配置的加载 / 校验 / 热重载。
//! 阶段 5 接入 serde + toml + notify（热键、多轮盘、条目、动画参数）。

/// 应用配置根（阶段 5 充实字段与持久化）。
#[derive(Debug, Default)]
pub struct Config;

#[cfg(test)]
mod tests {
    #[test]
    fn config_defaults_constructible() {
        let _ = super::Config::default();
    }
}
