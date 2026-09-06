//! 系统托盘。
//!
//! Windows / macOS 使用 `tray-icon` 原生实现；Linux 不做托盘（本模块为空壳），
//! 相关设置与关闭行为在 GUI 中自动降级。

/// 托盘菜单/点击发出的命令。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Command {
    /// 显示/隐藏主窗口。
    Toggle,
    /// 打开设置窗口。
    Settings,
    /// 退出应用。
    Quit,
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
pub struct Handle {
    /// 持有图标引用防止被系统移除。
    _icon: tray_icon::TrayIcon,
    rx: std::sync::mpsc::Receiver<Command>,
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
impl Handle {
    /// 创建托盘（需在原生事件循环已启动的主线程调用）。
    pub fn create(ctx: &egui::Context) -> Option<Self> {
        use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
        use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

        let menu = Menu::new();
        let toggle = MenuItem::with_id("toggle", "显示 / 隐藏主窗口", true, None);
        let settings = MenuItem::with_id("settings", "设置…", true, None);
        let quit = MenuItem::with_id("quit", "退出", true, None);
        menu.append_items(&[&toggle, &settings, &PredefinedMenuItem::separator(), &quit])
            .ok()?;

        let rgba = crate::icon::rgba_icon();
        let icon = Icon::from_rgba(rgba, crate::icon::SIZE, crate::icon::SIZE).ok()?;

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_icon(icon)
            .with_tooltip("ntrans")
            .with_menu_on_left_click(false)
            .build();
        let tray_icon = match tray_icon {
            Ok(t) => t,
            Err(e) => {
                eprintln!("创建托盘失败: {e}");
                return None;
            }
        };

        let (tx, rx) = std::sync::mpsc::channel::<Command>();

        // 菜单事件（在系统线程回调 → 转发到主线程 UI）。
        let tx_menu = tx.clone();
        let ctx_menu = ctx.clone();
        MenuEvent::set_event_handler(Some(Box::new(move |e: MenuEvent| {
            let cmd = match e.id().0.as_str() {
                "toggle" => Command::Toggle,
                "settings" => Command::Settings,
                "quit" => Command::Quit,
                _ => return,
            };
            let _ = tx_menu.send(cmd);
            ctx_menu.request_repaint();
        })));

        // 图标单击事件：切换窗口。
        let ctx_click = ctx.clone();
        TrayIconEvent::set_event_handler(Some(Box::new(move |e: TrayIconEvent| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = e
            {
                let _ = tx.send(Command::Toggle);
                ctx_click.request_repaint();
            }
        })));

        Some(Self {
            _icon: tray_icon,
            rx,
        })
    }

    /// 取出待处理的命令。
    pub fn pump(&self) -> Vec<Command> {
        self.rx.try_iter().collect()
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub struct Handle;

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
impl Handle {
    pub fn create(_ctx: &egui::Context) -> Option<Self> {
        None
    }

    pub fn pump(&self) -> Vec<Command> {
        Vec::new()
    }
}
