#[cfg(test)]
mod tests {
    use std::process::Command;

    #[test]
    fn test_tui_startup() {
        // 测试 TUI 可以启动
        let output = Command::new("../tui/target/release/clash-tui")
            .arg("--help")
            .output()
            .expect("Failed to execute TUI");

        assert!(output.status.success());
    }
}
