use ratatui::Frame;
use ratatui::layout::Rect;
use crossterm::event::KeyCode;

use crate::app::App;

/// 标签页 trait，定义所有标签页必须实现的方法
pub trait Tab {
    /// 标签页名称（用于显示在标签栏）
    fn name(&self) -> &str;

    /// 渲染标签页内容
    fn render(&self, frame: &mut Frame, area: Rect, app: &App);

    /// 处理键盘输入
    /// 返回 true 表示已处理，false 表示未处理
    fn handle_key(&self, key: KeyCode, app: &mut App) -> bool {
        false // 默认不处理
    }

    /// 获取帮助文本（显示在帮助覆盖层）
    fn help_text(&self) -> Option<&str> {
        None
    }

    /// 获取标签页特定的状态信息（用于状态栏）
    fn status_info(&self, app: &App) -> Option<String> {
        None
    }
}

/// 导出所有标签页
pub mod overview;
pub mod proxies;
pub mod subscriptions;
pub mod connections;
pub mod logs;

pub use overview::OverviewTab;
pub use proxies::ProxiesTab;
pub use subscriptions::SubscriptionsTab;
pub use connections::ConnectionsTab;
pub use logs::LogsTab;
