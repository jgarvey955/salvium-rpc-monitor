use crate::inventory::{
    ParamPreset, RpcContext, RpcField, RpcKind, RpcMethodSpec, daemon_default_method,
    daemon_method_names, default_method, find_method, input_strings_from_payload,
    is_read_only_method, load_inventory, method_names, parse_input_value, presets_for_method,
};
use crate::rpc::RpcBundle;
use crate::settings::Settings;
use chrono::Local;
use iced::widget::{button, column, container, pick_list, row, scrollable, text, text_input};
use iced::{
    Alignment, Background, Border, Color, Element, Fill, Shadow, Subscription, Task, Theme, window,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::time::Duration;

const ACCENT: Color = Color::from_rgb(0.96, 0.47, 0.12);
const ACCENT_DIM: Color = Color::from_rgb(0.82, 0.37, 0.10);
const BG_APP: Color = Color::from_rgb(0.07, 0.07, 0.08);
const BG_PANEL: Color = Color::from_rgb(0.11, 0.11, 0.12);
const BG_PANEL_ALT: Color = Color::from_rgb(0.15, 0.15, 0.17);
const BG_SIDEBAR: Color = Color::from_rgb(0.08, 0.08, 0.09);
const BORDER_SOFT: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.08);
const TEXT_MAIN: Color = Color::from_rgb(0.93, 0.93, 0.93);
const TEXT_MUTED: Color = Color::from_rgb(0.63, 0.65, 0.68);
const SUCCESS: Color = Color::from_rgb(0.21, 0.82, 0.47);
const DANGER: Color = Color::from_rgb(0.93, 0.27, 0.33);
const DEFAULT_ADDRESS: &str = "SC11UA22DFrAQerDwJwcf8Yh2ySTb7ipaFL8qSEX26tqUDdPf1RQBmmRuZG4SnRd8DNpp5vE1zDHnKNStiFDQsce49Q7fyp8Yp";
const DAEMON_NORMAL_PORT: &str = "19081";
const DAEMON_RESTRICTED_PORT: &str = "19089";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Home,
    Daemon,
    WalletRpc,
    Preferences,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Setup,
    Dashboard,
}

#[derive(Debug, Clone)]
pub enum Message {
    Refresh,
    StatusTick,
    SelectView(View),
    SelectDaemonMethod(String),
    SelectDaemonParam(String),
    SelectWalletMethod(String),
    SelectWalletParam(String),
    UpdateDaemonIp(String),
    UpdateDaemonPort(String),
    UpdateDaemonTransport(String),
    ToggleDaemonRestrictedMode,
    ToggleDaemonLoginEnabled,
    UpdateDaemonLoginUsername(String),
    UpdateDaemonLoginPassword(String),
    ToggleWalletEnabled,
    UpdateWalletIp(String),
    UpdateWalletPort(String),
    UpdateWalletTransport(String),
    ToggleWalletLoginEnabled,
    UpdateWalletLoginUsername(String),
    UpdateWalletLoginPassword(String),
    UpdateDaemonRequestField(String, String),
    UpdateWalletRequestField(String, String),
    PollDaemonSelection,
    PollWalletSelection,
    UpdatePollFrequency(String),
    SaveAndConnect,
    ExitRequested,
    ExitWindowResolved(Option<window::Id>),
}

#[derive(Debug, Clone)]
struct PollOutcome {
    daemon_polled: bool,
    daemon_status: String,
    daemon_version: Option<String>,
    daemon_height: Option<String>,
    current_block_height: Option<String>,
    target_height: Option<String>,
    nettype: Option<String>,
    peer_count: Option<String>,
    daemon_selected_output: Option<Value>,
    wallet_status: String,
    wallet_version: Option<String>,
    wallet_height: Option<String>,
    wallet_address: Option<String>,
    wallet_balance: Option<String>,
    wallet_selected_output: Option<Value>,
    wallet_polled: bool,
    raw_json: Value,
    notice: Option<String>,
    error: Option<String>,
}

pub struct AppState {
    screen: Screen,
    view: View,
    daemon_inventory: Vec<RpcMethodSpec>,
    wallet_inventory: Vec<RpcMethodSpec>,
    daemon_status: String,
    wallet_status: String,
    last_daemon_poll: String,
    last_wallet_poll: String,
    daemon_version: Option<String>,
    daemon_height: Option<String>,
    current_block_height: Option<String>,
    target_height: Option<String>,
    nettype: Option<String>,
    peer_count: Option<String>,
    wallet_version: Option<String>,
    wallet_height: Option<String>,
    wallet_address: Option<String>,
    wallet_balance: Option<String>,
    last_rpc_json: Option<Value>,
    error: Option<String>,
    notice: Option<String>,
    rpc: Option<RpcBundle>,
    daemon_method: Option<String>,
    daemon_param: Option<String>,
    wallet_method: Option<String>,
    wallet_param: Option<String>,
    selected_daemon_output: Option<Value>,
    selected_wallet_output: Option<Value>,
    daemon_ip_input: String,
    daemon_port_input: String,
    daemon_transport_input: String,
    daemon_restricted_mode: bool,
    daemon_login_enabled: bool,
    daemon_login_username_input: String,
    daemon_login_password_input: String,
    wallet_rpc_enabled: bool,
    wallet_ip_input: String,
    wallet_port_input: String,
    wallet_transport_input: String,
    wallet_login_enabled: bool,
    wallet_login_username_input: String,
    wallet_login_password_input: String,
    daemon_field_inputs: BTreeMap<String, String>,
    wallet_field_inputs: BTreeMap<String, String>,
    poll_frequency_input: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self::init()
    }
}

impl AppState {
    pub fn subscription(&self) -> Subscription<Message> {
        if self.screen != Screen::Dashboard {
            return Subscription::none();
        }

        let seconds = self
            .poll_frequency_input
            .trim()
            .parse::<u64>()
            .ok()
            .filter(|value| *value > 0)
            .unwrap_or(10);

        iced::time::every(Duration::from_secs(seconds)).map(|_| Message::StatusTick)
    }

    pub fn init() -> Self {
        let (settings, settings_exist, load_error) = match Settings::load() {
            Ok((settings, exists)) => (settings, exists, None),
            Err(error) => (
                Settings::default(),
                false,
                Some(format!("Failed to load settings: {error}")),
            ),
        };
        let daemon_inventory = load_inventory(RpcKind::Daemon);
        let wallet_inventory = load_inventory(RpcKind::Wallet);
        let daemon_method =
            daemon_default_method(&daemon_inventory, settings.daemon_restricted_mode);
        let wallet_method = default_method(RpcKind::Wallet, &wallet_inventory);

        let mut state = Self {
            screen: if settings_exist {
                Screen::Dashboard
            } else {
                Screen::Setup
            },
            view: View::Home,
            daemon_inventory,
            wallet_inventory,
            daemon_status: "Disconnected".into(),
            wallet_status: if settings.wallet_rpc_enabled {
                "Disconnected".into()
            } else {
                "Disabled".into()
            },
            last_daemon_poll: "Never".into(),
            last_wallet_poll: "Never".into(),
            daemon_version: None,
            daemon_height: None,
            current_block_height: None,
            target_height: None,
            nettype: None,
            peer_count: None,
            wallet_version: None,
            wallet_height: None,
            wallet_address: None,
            wallet_balance: None,
            last_rpc_json: None,
            error: load_error,
            notice: if settings_exist {
                None
            } else {
                Some(
                    "Enter the daemon settings, optionally enable wallet RPC, then verify access."
                        .into(),
                )
            },
            rpc: None,
            daemon_method,
            daemon_param: None,
            wallet_method,
            wallet_param: None,
            selected_daemon_output: None,
            selected_wallet_output: None,
            daemon_ip_input: settings.daemon_ip.clone(),
            daemon_port_input: settings.daemon_port.to_string(),
            daemon_transport_input: settings.daemon_transport.clone(),
            daemon_restricted_mode: settings.daemon_restricted_mode,
            daemon_login_enabled: settings.daemon_login_enabled,
            daemon_login_username_input: settings.daemon_login_username.clone(),
            daemon_login_password_input: settings.daemon_login_password.clone(),
            wallet_rpc_enabled: settings.wallet_rpc_enabled,
            wallet_ip_input: settings.wallet_ip.clone(),
            wallet_port_input: settings.wallet_port.to_string(),
            wallet_transport_input: settings.wallet_transport.clone(),
            wallet_login_enabled: settings.wallet_login_enabled,
            wallet_login_username_input: settings.wallet_login_username.clone(),
            wallet_login_password_input: settings.wallet_login_password.clone(),
            daemon_field_inputs: BTreeMap::new(),
            wallet_field_inputs: BTreeMap::new(),
            poll_frequency_input: settings.poll_frequency_seconds.to_string(),
        };

        state.ensure_daemon_method_selection();
        state.sync_param_selection(RpcKind::Wallet);
        state.refresh_request_inputs(RpcKind::Wallet);

        if settings_exist {
            match state.connect_with_current_inputs() {
                Ok(()) => {
                    state.notice =
                        Some("Saved settings loaded and connection checks completed.".into());
                }
                Err(error) => {
                    state.error = Some(error);
                    state.notice = Some("Preferences loaded, but one or more RPC endpoints are not reachable right now.".into());
                }
            }
        }

        state
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Refresh => {
                self.refresh_status();
                return Task::none();
            }
            Message::StatusTick => {
                self.refresh_status();
                return Task::none();
            }
            Message::SelectView(view) => {
                self.view = view;
                return Task::none();
            }
            Message::SelectDaemonMethod(method) => {
                self.daemon_method = Some(method);
                self.selected_daemon_output = None;
                self.sync_param_selection(RpcKind::Daemon);
                self.refresh_request_inputs(RpcKind::Daemon);
                return Task::none();
            }
            Message::SelectDaemonParam(param) => {
                self.daemon_param = Some(param);
                self.selected_daemon_output = None;
                self.refresh_request_inputs(RpcKind::Daemon);
                return Task::none();
            }
            Message::SelectWalletMethod(method) => {
                self.wallet_method = Some(method);
                self.selected_wallet_output = None;
                self.sync_param_selection(RpcKind::Wallet);
                self.refresh_request_inputs(RpcKind::Wallet);
                return Task::none();
            }
            Message::SelectWalletParam(param) => {
                self.wallet_param = Some(param);
                self.selected_wallet_output = None;
                self.refresh_request_inputs(RpcKind::Wallet);
                return Task::none();
            }
            Message::UpdateDaemonIp(value) => self.daemon_ip_input = value,
            Message::UpdateDaemonPort(value) => self.daemon_port_input = value,
            Message::UpdateDaemonTransport(value) => self.daemon_transport_input = value,
            Message::ToggleDaemonRestrictedMode => {
                self.daemon_restricted_mode = !self.daemon_restricted_mode;
                let current_port = self.daemon_port_input.trim();
                if current_port.is_empty()
                    || current_port == DAEMON_NORMAL_PORT
                    || current_port == DAEMON_RESTRICTED_PORT
                {
                    self.daemon_port_input = if self.daemon_restricted_mode {
                        DAEMON_RESTRICTED_PORT.to_string()
                    } else {
                        DAEMON_NORMAL_PORT.to_string()
                    };
                }
                self.ensure_daemon_method_selection();
                self.selected_daemon_output = None;
            }
            Message::ToggleDaemonLoginEnabled => {
                self.daemon_login_enabled = !self.daemon_login_enabled
            }
            Message::UpdateDaemonLoginUsername(value) => self.daemon_login_username_input = value,
            Message::UpdateDaemonLoginPassword(value) => self.daemon_login_password_input = value,
            Message::ToggleWalletEnabled => {
                self.wallet_rpc_enabled = !self.wallet_rpc_enabled;
                self.wallet_status = if self.wallet_rpc_enabled {
                    "Disconnected".into()
                } else {
                    "Disabled".into()
                };
            }
            Message::UpdateWalletIp(value) => self.wallet_ip_input = value,
            Message::UpdateWalletPort(value) => self.wallet_port_input = value,
            Message::UpdateWalletTransport(value) => self.wallet_transport_input = value,
            Message::ToggleWalletLoginEnabled => {
                self.wallet_login_enabled = !self.wallet_login_enabled
            }
            Message::UpdateWalletLoginUsername(value) => self.wallet_login_username_input = value,
            Message::UpdateWalletLoginPassword(value) => self.wallet_login_password_input = value,
            Message::UpdateDaemonRequestField(field, value) => {
                self.daemon_field_inputs.insert(field, value);
            }
            Message::UpdateWalletRequestField(field, value) => {
                self.wallet_field_inputs.insert(field, value);
            }
            Message::PollDaemonSelection => {
                self.manual_poll(RpcKind::Daemon);
                return Task::none();
            }
            Message::PollWalletSelection => {
                self.manual_poll(RpcKind::Wallet);
                return Task::none();
            }
            Message::UpdatePollFrequency(value) => self.poll_frequency_input = value,
            Message::SaveAndConnect => {
                match self.save_and_connect() {
                    Ok(()) => {
                        self.screen = Screen::Dashboard;
                        self.view = View::Home;
                    }
                    Err(error) => self.error = Some(error),
                }
                return Task::none();
            }
            Message::ExitRequested => return window::latest().map(Message::ExitWindowResolved),
            Message::ExitWindowResolved(Some(id)) => return window::close(id),
            Message::ExitWindowResolved(None) => return Task::none(),
        }

        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        match self.screen {
            Screen::Setup => self.setup_view(),
            Screen::Dashboard => self.dashboard_view(),
        }
    }

    fn setup_view(&self) -> Element<'_, Message> {
        let content = column![
            text("SALVIUM MONITOR").size(34).color(TEXT_MAIN),
            text("Initial RPC setup").size(16).color(ACCENT),
            text("The app verifies daemon access first and can also verify wallet RPC before opening the main page.")
                .size(15)
                .color(TEXT_MUTED),
            self.settings_editor(false),
        ]
        .spacing(18)
        .max_width(960);

        container(
            scrollable(content)
                .direction(default_vertical_scroll_direction())
                .style(content_scrollable_style)
                .height(Fill),
        )
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill)
        .padding(28)
        .into()
    }

    fn dashboard_view(&self) -> Element<'_, Message> {
        let title_bar = self.title_bar();
        let sidebar = self.sidebar();
        let content = match self.view {
            View::Home => self.home_view(),
            View::Daemon => self.rpc_view(RpcKind::Daemon),
            View::WalletRpc => self.rpc_view(RpcKind::Wallet),
            View::Preferences => self.preferences_view(),
        };

        let body = row![sidebar, content].spacing(18).height(Fill);

        container(column![title_bar, body].spacing(18).padding([10, 14]))
            .width(Fill)
            .height(Fill)
            .style(panel_style(BG_APP, Some(TEXT_MAIN), None))
            .into()
    }

    fn title_bar(&self) -> Element<'_, Message> {
        let daemon_badge = container(text(&self.daemon_status).size(13).color(TEXT_MAIN))
            .padding([6, 12])
            .style(panel_style(
                if self.daemon_status == "Connected" {
                    SUCCESS
                } else {
                    ACCENT_DIM
                },
                Some(TEXT_MAIN),
                None,
            ));
        let wallet_badge = container(text(&self.wallet_status).size(13).color(TEXT_MAIN))
            .padding([6, 12])
            .style(panel_style(
                if self.wallet_status == "Connected" {
                    SUCCESS
                } else {
                    ACCENT_DIM
                },
                Some(TEXT_MAIN),
                None,
            ));
        let status_group = row![
            column![text("Daemon").size(12).color(TEXT_MUTED), daemon_badge].spacing(6),
            column![text("Wallet").size(12).color(TEXT_MUTED), wallet_badge].spacing(6),
        ]
        .spacing(16)
        .align_y(Alignment::Center);

        let actions = row![
            self.menu_button(
                "Home",
                Message::SelectView(View::Home),
                self.view == View::Home
            ),
            self.menu_button(
                "Daemon",
                Message::SelectView(View::Daemon),
                self.view == View::Daemon
            ),
            self.menu_button(
                "Wallet RPC",
                Message::SelectView(View::WalletRpc),
                self.view == View::WalletRpc
            ),
            self.menu_button(
                "Preferences",
                Message::SelectView(View::Preferences),
                self.view == View::Preferences
            ),
            self.menu_button("Refresh", Message::Refresh, false),
            self.menu_button("Exit", Message::ExitRequested, false),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let bar = row![
            column![
                text("SALVIUM").size(14).color(ACCENT),
                text("Monitor").size(26).color(TEXT_MAIN),
            ]
            .spacing(2),
            container(status_group).width(Fill).center_x(Fill),
            actions,
        ]
        .align_y(Alignment::Center)
        .spacing(16);

        container(bar)
            .padding([16, 18])
            .style(panel_style(BG_PANEL, Some(TEXT_MAIN), Some(18.0)))
            .into()
    }

    fn sidebar(&self) -> Element<'_, Message> {
        let summary = container(
            column![
                text("SALVIUM").size(15).color(TEXT_MUTED),
                text("RPC Overview").size(24).color(TEXT_MAIN),
                self.metric_line("Daemon", &self.daemon_status),
                self.metric_line(
                    "Daemon Version",
                    &self.daemon_display_value(self.daemon_version.as_deref(), "Waiting")
                ),
                self.metric_line("Wallet RPC", &self.wallet_status),
                self.metric_line(
                    "Wallet Version",
                    self.wallet_version.as_deref().unwrap_or("Waiting")
                ),
                self.metric_line(
                    "Network",
                    &self.daemon_display_value(self.nettype.as_deref(), "Unknown")
                ),
            ]
            .spacing(10),
        )
        .padding(18)
        .style(panel_style(BG_PANEL_ALT, Some(TEXT_MAIN), Some(20.0)));

        let nav = column![
            self.nav_button("Home", View::Home),
            self.nav_button("Daemon RPC", View::Daemon),
            self.nav_button("Wallet RPC", View::WalletRpc),
            self.nav_button("Preferences", View::Preferences),
        ]
        .spacing(8);

        let footer = container(
            column![
                text("Daemon last poll").size(13).color(TEXT_MUTED),
                text(&self.last_daemon_poll).size(15).color(TEXT_MAIN),
                text("Wallet last poll").size(13).color(TEXT_MUTED),
                text(&self.last_wallet_poll).size(15).color(TEXT_MAIN),
                text(
                    self.notice
                        .as_deref()
                        .unwrap_or("Ready for manual refresh.")
                )
                .size(13)
                .color(TEXT_MUTED),
            ]
            .spacing(8),
        )
        .padding(16)
        .style(panel_style(BG_PANEL, Some(TEXT_MAIN), Some(16.0)));

        container(column![summary, nav, footer].spacing(16))
            .width(300)
            .height(Fill)
            .style(panel_style(BG_SIDEBAR, Some(TEXT_MAIN), Some(24.0)))
            .padding(12)
            .into()
    }

    fn home_view(&self) -> Element<'_, Message> {
        let settings = self.settings_snapshot();
        let daemon_endpoint = settings.daemon_url_display();
        let wallet_endpoint = if self.wallet_rpc_enabled {
            settings.wallet_url_display()
        } else {
            "Wallet RPC disabled".to_string()
        };

        let metrics = row![
            self.info_card(
                "Daemon Version",
                &self.daemon_display_value(self.daemon_version.as_deref(), "No response")
            ),
            self.info_card(
                "Daemon Height",
                &self.daemon_display_value(self.daemon_height.as_deref(), "Unknown")
            ),
            self.info_card(
                "Wallet Height",
                self.wallet_height.as_deref().unwrap_or("Unknown")
            ),
            self.info_card(
                "Wallet Balance",
                self.wallet_balance.as_deref().unwrap_or("Unknown")
            ),
        ]
        .spacing(14);

        let status = container(
            column![
                text("Home").size(28).color(TEXT_MAIN),
                text("The monitor now tracks daemon and wallet RPC independently, including auth and TLS settings.")
                    .size(15)
                    .color(TEXT_MUTED),
                self.value_box("Daemon endpoint", daemon_endpoint),
                self.value_box("Wallet endpoint", wallet_endpoint),
                self.summary_grid(),
                self.message_panel(),
            ]
            .spacing(16),
        )
        .padding(22)
        .style(panel_style(BG_PANEL, Some(TEXT_MAIN), Some(22.0)));

        container(column![metrics, status].spacing(16))
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn rpc_view(&self, kind: RpcKind) -> Element<'_, Message> {
        let (
            title,
            method_options,
            selected_method,
            param_options,
            selected_param,
            input_editor,
            output,
            summary,
        ) = match kind {
            RpcKind::Daemon => (
                "Daemon",
                self.daemon_method_options(),
                self.daemon_method.clone(),
                self.daemon_param_options(),
                self.daemon_param.clone(),
                self.request_fields_editor(RpcKind::Daemon),
                self.selected_daemon_output(),
                self.daemon_summary_grid(),
            ),
            RpcKind::Wallet => (
                "Wallet RPC",
                self.wallet_method_options(),
                self.wallet_method.clone(),
                self.wallet_param_options(),
                self.wallet_param.clone(),
                self.request_fields_editor(RpcKind::Wallet),
                self.selected_wallet_output(),
                self.wallet_summary_grid(),
            ),
        };

        let method_picker = match kind {
            RpcKind::Daemon => {
                pick_list(method_options, selected_method, Message::SelectDaemonMethod)
            }
            RpcKind::Wallet => {
                pick_list(method_options, selected_method, Message::SelectWalletMethod)
            }
        }
        .placeholder("Select RPC method")
        .padding([10, 14])
        .text_size(15)
        .style(daemon_pick_list_style);

        let param_picker = match kind {
            RpcKind::Daemon => pick_list(param_options, selected_param, Message::SelectDaemonParam),
            RpcKind::Wallet => pick_list(param_options, selected_param, Message::SelectWalletParam),
        }
        .placeholder("Select params")
        .padding([10, 14])
        .text_size(15)
        .style(daemon_pick_list_style);

        let header = container(
            column![
                row![
                    text(title).size(28).color(TEXT_MAIN),
                    method_picker,
                    self.menu_button(
                        "Poll",
                        match kind {
                            RpcKind::Daemon => Message::PollDaemonSelection,
                            RpcKind::Wallet => Message::PollWalletSelection,
                        },
                        false
                    ),
                ]
                .spacing(18)
                .align_y(Alignment::Center),
                row![text("Template").size(15).color(TEXT_MUTED), param_picker]
                    .spacing(18)
                    .align_y(Alignment::Center),
                text(match kind {
                    RpcKind::Daemon =>
                        "Methods and parameter templates are loaded from rpc.output.",
                    RpcKind::Wallet =>
                        "Methods and parameter templates are loaded from walletrpc.output.",
                })
                .size(15)
                .color(TEXT_MUTED),
                input_editor,
                summary,
            ]
            .spacing(14),
        )
        .padding(22)
        .style(panel_style(BG_PANEL, Some(TEXT_MAIN), Some(22.0)));

        let raw_panel = container(
            scrollable(
                container(text(output).size(14).color(TEXT_MAIN))
                    .padding(18)
                    .style(panel_style(BG_PANEL_ALT, Some(TEXT_MAIN), Some(18.0))),
            )
            .direction(default_vertical_scroll_direction())
            .style(content_scrollable_style)
            .spacing(10)
            .height(Fill)
            .width(Fill),
        )
        .height(Fill)
        .style(panel_style(BG_PANEL, Some(TEXT_MAIN), Some(22.0)))
        .padding(18);

        scrollable(column![header, raw_panel].spacing(16))
            .direction(default_vertical_scroll_direction())
            .style(content_scrollable_style)
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn preferences_view(&self) -> Element<'_, Message> {
        container(
            scrollable(
                container(
                    column![
                        text("Preferences").size(28).color(TEXT_MAIN),
                        text("Daemon and wallet RPC connection details are stored here, including login and TLS handling.")
                            .size(15)
                            .color(TEXT_MUTED),
                        self.settings_editor(true),
                    ]
                    .spacing(18),
                )
                .padding(22)
            )
            .direction(default_vertical_scroll_direction())
            .style(content_scrollable_style)
            .width(Fill)
            .height(Fill),
        )
        .width(Fill)
        .height(Fill)
        .style(panel_style(BG_PANEL, Some(TEXT_MAIN), Some(22.0)))
        .into()
    }

    fn settings_editor(&self, preferences_mode: bool) -> Element<'_, Message> {
        let title = if preferences_mode {
            "Update RPC settings and reconnect"
        } else {
            "Connection settings"
        };

        let action_label = if preferences_mode {
            "Save Settings"
        } else {
            "Verify and Continue"
        };

        let daemon_section = container(
            column![
                text("Daemon RPC").size(20).color(TEXT_MAIN),
                row![
                    self.field(
                        "Daemon IP",
                        "192.168.0.30",
                        &self.daemon_ip_input,
                        Message::UpdateDaemonIp
                    ),
                    self.field(
                        "Daemon Port",
                        DAEMON_NORMAL_PORT,
                        &self.daemon_port_input,
                        Message::UpdateDaemonPort
                    ),
                ]
                .spacing(14),
                self.transport_field(
                    "Transport",
                    &self.daemon_transport_input,
                    Message::UpdateDaemonTransport
                ),
                self.toggle_field(
                    "Daemon Restricted Mode",
                    self.daemon_restricted_mode,
                    Message::ToggleDaemonRestrictedMode
                ),
                row![
                    self.toggle_field(
                        "RPC Login Enabled",
                        self.daemon_login_enabled,
                        Message::ToggleDaemonLoginEnabled
                    ),
                    self.field(
                        "RPC Username",
                        "optional",
                        &self.daemon_login_username_input,
                        Message::UpdateDaemonLoginUsername
                    ),
                ]
                .spacing(14),
                self.field(
                    "RPC Password",
                    "optional",
                    &self.daemon_login_password_input,
                    Message::UpdateDaemonLoginPassword
                ),
            ]
            .spacing(14),
        )
        .padding(20)
        .style(panel_style(BG_PANEL_ALT, Some(TEXT_MAIN), Some(20.0)));

        let wallet_section = container(
            column![
                text("Wallet RPC").size(20).color(TEXT_MAIN),
                self.toggle_field(
                    "Wallet RPC Enabled",
                    self.wallet_rpc_enabled,
                    Message::ToggleWalletEnabled
                ),
                row![
                    self.field(
                        "Wallet IP",
                        "192.168.0.30",
                        &self.wallet_ip_input,
                        Message::UpdateWalletIp
                    ),
                    self.field(
                        "Wallet Port",
                        "19092",
                        &self.wallet_port_input,
                        Message::UpdateWalletPort
                    ),
                ]
                .spacing(14),
                self.transport_field(
                    "Transport",
                    &self.wallet_transport_input,
                    Message::UpdateWalletTransport
                ),
                row![
                    self.toggle_field(
                        "RPC Login Enabled",
                        self.wallet_login_enabled,
                        Message::ToggleWalletLoginEnabled
                    ),
                    self.field(
                        "RPC Username",
                        "salviumuser",
                        &self.wallet_login_username_input,
                        Message::UpdateWalletLoginUsername
                    ),
                ]
                .spacing(14),
                self.field(
                    "RPC Password",
                    "wallet rpc password",
                    &self.wallet_login_password_input,
                    Message::UpdateWalletLoginPassword
                ),
            ]
            .spacing(14),
        )
        .padding(20)
        .style(panel_style(BG_PANEL_ALT, Some(TEXT_MAIN), Some(20.0)));

        let editor = column![
            text(title).size(22).color(TEXT_MAIN),
            daemon_section,
            wallet_section,
            self.field(
                "Poll Frequency (seconds)",
                "10",
                &self.poll_frequency_input,
                Message::UpdatePollFrequency
            ),
            self.message_panel(),
            button(text(action_label).size(16))
                .padding([12, 20])
                .style(primary_button_style)
                .on_press(Message::SaveAndConnect),
        ]
        .spacing(16);

        container(editor)
            .padding(20)
            .style(panel_style(BG_PANEL_ALT, Some(TEXT_MAIN), Some(20.0)))
            .into()
    }

    fn field(
        &self,
        label: &'static str,
        placeholder: &'static str,
        value: &str,
        on_input: fn(String) -> Message,
    ) -> Element<'_, Message> {
        let input = text_input(placeholder, value)
            .on_input(on_input)
            .padding(12)
            .size(16)
            .style(input_style);

        container(column![text(label).size(13).color(TEXT_MUTED), input].spacing(8))
            .width(Fill)
            .into()
    }

    fn transport_field(
        &self,
        label: &'static str,
        selected: &str,
        on_selected: fn(String) -> Message,
    ) -> Element<'_, Message> {
        let options = transport_options();
        let current = options
            .iter()
            .find(|option| option.as_str() == selected)
            .cloned();

        container(
            column![
                text(label).size(13).color(TEXT_MUTED),
                pick_list(options, current, on_selected)
                    .placeholder("Select transport")
                    .padding([10, 14])
                    .text_size(15)
                    .style(daemon_pick_list_style),
            ]
            .spacing(8),
        )
        .width(Fill)
        .into()
    }

    fn request_fields_editor(&self, kind: RpcKind) -> Element<'_, Message> {
        let Some(method) = self.selected_method_spec(kind) else {
            return container(
                text("Select an RPC method to load inputs.")
                    .size(14)
                    .color(TEXT_MUTED),
            )
            .padding(16)
            .style(panel_style(BG_PANEL_ALT, Some(TEXT_MAIN), Some(16.0)))
            .into();
        };

        if method.request_fields.is_empty() {
            return container(
                text("This method does not require any request fields.")
                    .size(14)
                    .color(TEXT_MUTED),
            )
            .padding(16)
            .style(panel_style(BG_PANEL_ALT, Some(TEXT_MAIN), Some(16.0)))
            .into();
        }

        let mut fields = column![text("Request Inputs").size(18).color(TEXT_MAIN)].spacing(12);

        for field in &method.request_fields {
            fields = fields.push(self.request_field_input(kind, field));
        }

        container(fields)
            .padding(18)
            .style(panel_style(BG_PANEL_ALT, Some(TEXT_MAIN), Some(18.0)))
            .into()
    }

    fn request_field_input(&self, kind: RpcKind, field: &RpcField) -> Element<'_, Message> {
        let value = self
            .field_input_map(kind)
            .get(&field.name)
            .cloned()
            .unwrap_or_default();
        let label = format!("{} ({})", field.name, field.ty);
        let field_name = field.name.clone();

        let input = match kind {
            RpcKind::Daemon => text_input(&field.name, &value).on_input(move |value| {
                Message::UpdateDaemonRequestField(field_name.clone(), value)
            }),
            RpcKind::Wallet => text_input(&field.name, &value).on_input(move |value| {
                Message::UpdateWalletRequestField(field_name.clone(), value)
            }),
        }
        .padding(12)
        .size(15)
        .style(input_style);

        container(column![text(label).size(13).color(TEXT_MUTED), input].spacing(8))
            .width(Fill)
            .into()
    }

    fn toggle_field(
        &self,
        label: &'static str,
        enabled: bool,
        message: Message,
    ) -> Element<'_, Message> {
        container(
            column![
                text(label).size(13).color(TEXT_MUTED),
                button(
                    text(if enabled { "Enabled" } else { "Disabled" })
                        .size(15)
                        .color(TEXT_MAIN)
                )
                .padding([12, 14])
                .style(move |_theme, status| top_button_style(enabled, status))
                .on_press(message),
            ]
            .spacing(8),
        )
        .width(Fill)
        .into()
    }

    fn info_card<'a>(&self, label: &'a str, value: impl Into<String>) -> Element<'a, Message> {
        let value = value.into();
        container(
            column![
                text(label).size(13).color(TEXT_MUTED),
                text(value).size(24).color(TEXT_MAIN)
            ]
            .spacing(10),
        )
        .width(Fill)
        .padding(18)
        .style(panel_style(BG_PANEL, Some(TEXT_MAIN), Some(18.0)))
        .into()
    }

    fn value_box(&self, label: &'static str, value: String) -> Element<'_, Message> {
        container(
            column![
                text(label).size(13).color(TEXT_MUTED),
                container(text(value).size(18).color(TEXT_MAIN))
                    .padding([12, 14])
                    .style(panel_style(BG_PANEL_ALT, Some(TEXT_MAIN), Some(14.0))),
            ]
            .spacing(8),
        )
        .into()
    }

    fn summary_grid(&self) -> Element<'_, Message> {
        let left = column![
            self.metric_line("Daemon", self.connection_label()),
            self.metric_line(
                "Target Height",
                &self.daemon_display_value(self.target_height.as_deref(), "Unknown")
            ),
            self.metric_line(
                "Peer Count",
                &self.daemon_display_value(self.peer_count.as_deref(), "Unknown")
            ),
        ]
        .spacing(10);

        let right = column![
            self.metric_line("Wallet RPC", &self.wallet_status),
            self.metric_line(
                "Network Type",
                &self.daemon_display_value(self.nettype.as_deref(), "Unknown")
            ),
        ]
        .spacing(10);

        container(
            column![
                row![left, right].spacing(28),
                self.long_value_line(
                    "Wallet Address",
                    self.wallet_address.as_deref().unwrap_or("Unknown")
                ),
            ]
            .spacing(14),
        )
        .padding(18)
        .style(panel_style(BG_PANEL_ALT, Some(TEXT_MAIN), Some(16.0)))
        .into()
    }

    fn daemon_summary_grid(&self) -> Element<'_, Message> {
        container(
            row![
                column![
                    self.metric_line("Connection", self.connection_label()),
                    self.metric_line(
                        "Current Block Height",
                        &self.daemon_display_value(
                            self.current_block_height
                                .as_deref()
                                .or(self.daemon_height.as_deref()),
                            "Unknown"
                        )
                    ),
                ]
                .spacing(10),
                column![
                    self.metric_line(
                        "Network Type",
                        &self.daemon_display_value(self.nettype.as_deref(), "Unknown")
                    ),
                    self.metric_line(
                        "Peer Count",
                        &self.daemon_display_value(self.peer_count.as_deref(), "Unknown")
                    ),
                ]
                .spacing(10),
            ]
            .spacing(28),
        )
        .padding(18)
        .style(panel_style(BG_PANEL_ALT, Some(TEXT_MAIN), Some(16.0)))
        .into()
    }

    fn wallet_summary_grid(&self) -> Element<'_, Message> {
        container(
            column![
                row![
                    column![
                        self.metric_line("Wallet RPC", &self.wallet_status),
                        self.metric_line(
                            "Wallet Height",
                            self.wallet_height.as_deref().unwrap_or("Unknown")
                        ),
                    ]
                    .spacing(10),
                    column![self.metric_line(
                        "Wallet Balance",
                        self.wallet_balance.as_deref().unwrap_or("Unknown")
                    ),]
                    .spacing(10),
                ]
                .spacing(28),
                self.long_value_line(
                    "Wallet Address",
                    self.wallet_address.as_deref().unwrap_or("Unknown")
                ),
            ]
            .spacing(14),
        )
        .padding(18)
        .style(panel_style(BG_PANEL_ALT, Some(TEXT_MAIN), Some(16.0)))
        .into()
    }

    fn metric_line<'a>(&self, label: &'a str, value: impl Into<String>) -> Element<'a, Message> {
        let value = value.into();
        row![
            text(label).size(13).color(TEXT_MUTED),
            container(text(value).size(15).color(TEXT_MAIN))
                .width(Fill)
                .align_right(Fill),
        ]
        .align_y(Alignment::Center)
        .into()
    }

    fn daemon_display_value(&self, value: Option<&str>, fallback: &str) -> String {
        value.map(ToOwned::to_owned).unwrap_or_else(|| {
            if self.daemon_restricted_mode && self.daemon_status == "Connected" {
                "Restricted mode active".to_string()
            } else {
                fallback.to_string()
            }
        })
    }

    fn long_value_line<'a>(&self, label: &'a str, value: &'a str) -> Element<'a, Message> {
        container(
            column![
                text(label).size(13).color(TEXT_MUTED),
                container(
                    text(value)
                        .size(13)
                        .color(TEXT_MAIN)
                        .wrapping(iced::widget::text::Wrapping::None)
                )
                .width(Fill)
                .padding([12, 14])
                .style(panel_style(BG_PANEL, Some(TEXT_MAIN), Some(14.0))),
            ]
            .spacing(8),
        )
        .width(Fill)
        .into()
    }

    fn message_panel(&self) -> Element<'_, Message> {
        let notice = self
            .notice
            .as_deref()
            .map(|message| text(message).size(14).color(TEXT_MUTED));
        let error = self
            .error
            .as_deref()
            .map(|message| text(message).size(14).color(DANGER));

        let content = match (notice, error) {
            (Some(notice), Some(error)) => column![notice, error].spacing(8),
            (Some(notice), None) => column![notice],
            (None, Some(error)) => column![error],
            (None, None) => column![text("No warnings.").size(14).color(TEXT_MUTED)],
        };

        container(content)
            .padding(14)
            .style(panel_style(BG_PANEL, Some(TEXT_MAIN), Some(14.0)))
            .into()
    }

    fn nav_button(&self, label: &'static str, view: View) -> Element<'_, Message> {
        let active = self.view == view;
        button(
            row![
                text(label)
                    .size(18)
                    .color(if active { TEXT_MAIN } else { TEXT_MUTED }),
                container(
                    text(">")
                        .size(16)
                        .color(if active { ACCENT } else { TEXT_MUTED })
                )
                .width(Fill)
                .align_right(Fill),
            ]
            .align_y(Alignment::Center),
        )
        .width(Fill)
        .padding([14, 16])
        .style(move |_theme, status| sidebar_button_style(active, status))
        .on_press(Message::SelectView(view))
        .into()
    }

    fn menu_button(
        &self,
        label: &'static str,
        message: Message,
        active: bool,
    ) -> Element<'_, Message> {
        button(text(label).size(14).color(TEXT_MAIN))
            .padding([8, 12])
            .style(move |_theme, status| top_button_style(active, status))
            .on_press(message)
            .into()
    }

    fn connection_label(&self) -> &str {
        if self.rpc.is_some() {
            "Connected"
        } else {
            "Disconnected"
        }
    }

    fn sync_param_selection(&mut self, kind: RpcKind) {
        let first_param = self
            .presets_for_selected_method(kind)
            .first()
            .map(|preset| preset.key.clone());

        match kind {
            RpcKind::Daemon => self.daemon_param = first_param,
            RpcKind::Wallet => self.wallet_param = first_param,
        }
    }

    fn refresh_status(&mut self) {
        self.error = None;
        self.notice = None;

        match self.settings_from_inputs() {
            Ok(settings) => match Self::poll_with_settings(
                &settings,
                false,
                None,
                None,
                &self.daemon_field_inputs,
                None,
                None,
                &self.wallet_field_inputs,
                &self.daemon_inventory,
                &self.wallet_inventory,
            ) {
                Ok((bundle, outcome)) => self.apply_poll(bundle, outcome),
                Err(error) => {
                    self.rpc = None;
                    self.daemon_status = "Disconnected".into();
                    self.wallet_status = if self.wallet_rpc_enabled {
                        "Disconnected".into()
                    } else {
                        "Disabled".into()
                    };
                    self.notice = Some("RPC status check failed.".into());
                    self.error = Some(error);
                }
            },
            Err(error) => self.error = Some(error),
        }
    }

    fn manual_poll(&mut self, kind: RpcKind) {
        self.error = None;
        self.notice = None;

        match self.settings_from_inputs() {
            Ok(settings) => {
                let result = match kind {
                    RpcKind::Daemon => Self::poll_with_settings(
                        &settings,
                        false,
                        self.daemon_method.as_deref(),
                        self.daemon_param.as_deref(),
                        &self.daemon_field_inputs,
                        None,
                        None,
                        &self.wallet_field_inputs,
                        &self.daemon_inventory,
                        &self.wallet_inventory,
                    ),
                    RpcKind::Wallet => Self::poll_with_settings(
                        &settings,
                        true,
                        None,
                        None,
                        &self.daemon_field_inputs,
                        self.wallet_method.as_deref(),
                        self.wallet_param.as_deref(),
                        &self.wallet_field_inputs,
                        &self.daemon_inventory,
                        &self.wallet_inventory,
                    ),
                };

                match result {
                    Ok((bundle, outcome)) => self.apply_poll(bundle, outcome),
                    Err(error) => {
                        let error_message = error.clone();
                        self.error = Some(error);
                        self.notice = Some(match kind {
                            RpcKind::Daemon => "Daemon RPC poll failed.".into(),
                            RpcKind::Wallet => "Wallet RPC poll failed.".into(),
                        });
                        match kind {
                            RpcKind::Daemon => {
                                self.selected_daemon_output =
                                    Some(json!({ "error": error_message }));
                            }
                            RpcKind::Wallet => {
                                self.selected_wallet_output =
                                    Some(json!({ "error": error_message }));
                            }
                        }
                    }
                }
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn save_and_connect(&mut self) -> Result<(), String> {
        self.error = None;
        self.notice = None;
        let settings = self.settings_from_inputs()?;
        let (bundle, outcome) = Self::poll_with_settings(
            &settings,
            true,
            None,
            None,
            &self.daemon_field_inputs,
            None,
            None,
            &self.wallet_field_inputs,
            &self.daemon_inventory,
            &self.wallet_inventory,
        )?;

        settings
            .save()
            .map_err(|error| format!("Failed to save settings.json: {error}"))?;

        self.notice = Some("Connection verified and settings saved.".into());
        self.apply_poll(bundle, outcome);
        Ok(())
    }

    fn connect_with_current_inputs(&mut self) -> Result<(), String> {
        self.error = None;
        let settings = self.settings_from_inputs()?;
        let (bundle, outcome) = Self::poll_with_settings(
            &settings,
            false,
            None,
            None,
            &self.daemon_field_inputs,
            None,
            None,
            &self.wallet_field_inputs,
            &self.daemon_inventory,
            &self.wallet_inventory,
        )?;
        self.apply_poll(bundle, outcome);
        Ok(())
    }

    fn settings_from_inputs(&self) -> Result<Settings, String> {
        let daemon_port =
            self.daemon_port_input.trim().parse::<u16>().map_err(|_| {
                "Daemon port must be a valid number between 0 and 65535.".to_string()
            })?;
        let wallet_port =
            self.wallet_port_input.trim().parse::<u16>().map_err(|_| {
                "Wallet port must be a valid number between 0 and 65535.".to_string()
            })?;
        let poll_frequency_seconds = self
            .poll_frequency_input
            .trim()
            .parse::<u64>()
            .map_err(|_| "Poll frequency must be a valid number of seconds.".to_string())?;

        if self.daemon_ip_input.trim().is_empty() {
            return Err("Daemon IP cannot be empty.".into());
        }

        if self.wallet_rpc_enabled && self.wallet_ip_input.trim().is_empty() {
            return Err("Wallet IP cannot be empty when wallet RPC is enabled.".into());
        }

        if self.daemon_login_enabled {
            if self.daemon_login_username_input.trim().is_empty() {
                return Err(
                    "Daemon RPC login is enabled, so the daemon username is required.".into(),
                );
            }
            if self.daemon_login_password_input.trim().is_empty() {
                return Err(
                    "Daemon RPC login is enabled, so the daemon password is required.".into(),
                );
            }
        }

        if self.wallet_rpc_enabled && self.wallet_login_enabled {
            if self.wallet_login_username_input.trim().is_empty() {
                return Err(
                    "Wallet RPC login is enabled, so the wallet username is required.".into(),
                );
            }
            if self.wallet_login_password_input.trim().is_empty() {
                return Err(
                    "Wallet RPC login is enabled, so the wallet password is required.".into(),
                );
            }
        }

        if poll_frequency_seconds == 0 {
            return Err("Poll frequency must be greater than zero.".into());
        }

        Ok(Settings {
            daemon_ip: self.daemon_ip_input.trim().to_string(),
            daemon_port,
            daemon_transport: normalize_transport(&self.daemon_transport_input, "http"),
            daemon_restricted_mode: self.daemon_restricted_mode,
            daemon_login_enabled: self.daemon_login_enabled,
            daemon_login_username: self.daemon_login_username_input.trim().to_string(),
            daemon_login_password: self.daemon_login_password_input.trim().to_string(),
            wallet_rpc_enabled: self.wallet_rpc_enabled,
            wallet_ip: self.wallet_ip_input.trim().to_string(),
            wallet_port,
            wallet_transport: normalize_transport(&self.wallet_transport_input, "https"),
            wallet_login_enabled: self.wallet_login_enabled,
            wallet_login_username: self.wallet_login_username_input.trim().to_string(),
            wallet_login_password: self.wallet_login_password_input.trim().to_string(),
            poll_frequency_seconds,
        })
    }

    fn settings_snapshot(&self) -> Settings {
        self.settings_from_inputs()
            .unwrap_or_else(|_| Settings::default())
    }

    fn daemon_method_options(&self) -> Vec<String> {
        daemon_method_names(&self.daemon_inventory, self.daemon_restricted_mode)
    }

    fn wallet_method_options(&self) -> Vec<String> {
        method_names(&self.wallet_inventory)
    }

    fn ensure_daemon_method_selection(&mut self) {
        let options = self.daemon_method_options();
        let current_valid = self
            .daemon_method
            .as_deref()
            .map(|method| options.iter().any(|option| option == method))
            .unwrap_or(false);

        if !current_valid {
            self.daemon_method =
                daemon_default_method(&self.daemon_inventory, self.daemon_restricted_mode);
        }

        self.sync_param_selection(RpcKind::Daemon);
        self.refresh_request_inputs(RpcKind::Daemon);
    }

    fn daemon_param_options(&self) -> Vec<String> {
        self.presets_for_selected_method(RpcKind::Daemon)
            .into_iter()
            .map(|preset| preset.key)
            .collect()
    }

    fn wallet_param_options(&self) -> Vec<String> {
        self.presets_for_selected_method(RpcKind::Wallet)
            .into_iter()
            .map(|preset| preset.key)
            .collect()
    }

    fn presets_for_selected_method(&self, kind: RpcKind) -> Vec<ParamPreset> {
        let context = RpcContext {
            current_height: self
                .current_block_height
                .as_deref()
                .and_then(|value| value.parse::<u64>().ok())
                .or_else(|| {
                    self.daemon_height
                        .as_deref()
                        .and_then(|value| value.parse::<u64>().ok())
                }),
            current_hash: self
                .last_rpc_json
                .as_ref()
                .and_then(|value| value.get("current_block"))
                .and_then(|value| value.get("result"))
                .and_then(|value| value.get("block_header"))
                .and_then(|value| value.get("hash"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            wallet_address: self
                .wallet_address
                .clone()
                .or_else(|| Some(DEFAULT_ADDRESS.to_string())),
        };

        match kind {
            RpcKind::Daemon => self
                .daemon_method
                .as_deref()
                .and_then(|method| find_method(&self.daemon_inventory, method))
                .map(|method| presets_for_method(RpcKind::Daemon, method, &context))
                .unwrap_or_default(),
            RpcKind::Wallet => self
                .wallet_method
                .as_deref()
                .and_then(|method| find_method(&self.wallet_inventory, method))
                .map(|method| presets_for_method(RpcKind::Wallet, method, &context))
                .unwrap_or_default(),
        }
    }

    fn poll_with_settings(
        settings: &Settings,
        require_wallet: bool,
        daemon_method: Option<&str>,
        daemon_param: Option<&str>,
        daemon_inputs: &BTreeMap<String, String>,
        wallet_method: Option<&str>,
        wallet_param: Option<&str>,
        wallet_inputs: &BTreeMap<String, String>,
        daemon_inventory: &[RpcMethodSpec],
        wallet_inventory: &[RpcMethodSpec],
    ) -> Result<(RpcBundle, PollOutcome), String> {
        let bundle = RpcBundle::from_settings(settings)?;
        let (
            daemon_status,
            daemon_version,
            daemon_height,
            current_block_height,
            target_height,
            nettype,
            peer_count,
            current_block,
            daemon_raw_json,
        ) = if settings.daemon_restricted_mode {
            let version = bundle.daemon.json_rpc("get_version", Value::Null)?;
            validate_rpc_status(&version, "get_version")?;
            let block_count = bundle
                .daemon
                .json_rpc("get_block_count", Value::Null)
                .or_else(|_| bundle.daemon.json_rpc("getblockcount", Value::Null))?;
            validate_rpc_status(&block_count, "get_block_count")?;

            let height = block_count
                .get("result")
                .and_then(|result| result.get("count"))
                .and_then(as_u64_string)
                .or_else(|| {
                    version
                        .get("result")
                        .and_then(|result| result.get("current_height"))
                        .and_then(as_u64_string)
                });

            (
                "Connected".to_string(),
                version
                    .get("result")
                    .and_then(|result| result.get("version"))
                    .map(render_json_value),
                height.clone(),
                height,
                version
                    .get("result")
                    .and_then(|result| result.get("target_height"))
                    .map(render_json_value),
                None,
                None,
                Value::Null,
                json!({
                    "get_version": version,
                    "get_block_count": block_count,
                }),
            )
        } else {
            let daemon_info = bundle.daemon.json_rpc("get_info", Value::Null)?;
            validate_rpc_status(&daemon_info, "get_info")?;
            let daemon_info_result = daemon_info.get("result").cloned().unwrap_or(Value::Null);
            let current_block_height = daemon_info_result
                .get("height")
                .or_else(|| daemon_info_result.get("block_height"))
                .and_then(as_u64_string);
            let top_block_height = current_block_height
                .as_deref()
                .and_then(|value| value.parse::<u64>().ok())
                .and_then(|height| height.checked_sub(1));
            let current_block = match top_block_height {
                Some(height) => {
                    let value = bundle
                        .daemon
                        .call("get_block_header_by_height", json!({ "height": height }))?;
                    validate_rpc_status(&value, "get_block_header_by_height")?;
                    value
                }
                None => Value::Null,
            };

            (
                "Connected".to_string(),
                daemon_info_result.get("version").map(render_json_value),
                current_block_height.clone(),
                current_block_height,
                daemon_info_result
                    .get("target_height")
                    .or_else(|| daemon_info_result.get("height_without_bootstrap"))
                    .map(render_json_value),
                daemon_info_result
                    .get("nettype")
                    .or_else(|| daemon_info_result.get("network_type"))
                    .map(render_json_value),
                match (
                    daemon_info_result.get("incoming_connections_count"),
                    daemon_info_result.get("outgoing_connections_count"),
                ) {
                    (Some(incoming), Some(outgoing)) => Some(format!(
                        "{} in / {} out",
                        render_json_value(incoming),
                        render_json_value(outgoing)
                    )),
                    _ => None,
                },
                current_block,
                json!({
                    "get_info": daemon_info,
                }),
            )
        };

        let mut context = RpcContext {
            current_height: current_block_height
                .as_deref()
                .and_then(|value| value.parse::<u64>().ok())
                .and_then(|height| height.checked_sub(1)),
            current_hash: current_block
                .get("result")
                .and_then(|result| result.get("block_header"))
                .and_then(|header| header.get("hash"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            wallet_address: Some(DEFAULT_ADDRESS.to_string()),
        };

        let daemon_selected_output = if let Some(method_name) = daemon_method {
            if let Some(spec) = find_method(daemon_inventory, method_name) {
                let preset = presets_for_method(RpcKind::Daemon, spec, &context)
                    .into_iter()
                    .find(|preset| Some(preset.key.as_str()) == daemon_param)
                    .or_else(|| {
                        presets_for_method(RpcKind::Daemon, spec, &context)
                            .into_iter()
                            .next()
                    });
                Some(Self::poll_selected_method(
                    &bundle,
                    RpcKind::Daemon,
                    spec,
                    preset.as_ref(),
                    daemon_inputs,
                    &mut context,
                )?)
            } else {
                None
            }
        } else {
            None
        };

        let mut wallet_json = Value::Null;
        let mut wallet_version = None;
        let mut wallet_height = None;
        let mut wallet_address = None;
        let mut wallet_balance = None;
        let mut wallet_selected_output = None;
        let mut wallet_status = if settings.wallet_rpc_enabled {
            "Disconnected".to_string()
        } else {
            "Disabled".to_string()
        };
        let mut wallet_error = None;

        if let Some(wallet) = &bundle.wallet {
            match (|| -> Result<(), String> {
                let version = wallet.json_rpc("get_version", Value::Null)?;
                let height = wallet
                    .json_rpc("get_height", Value::Null)
                    .or_else(|_| wallet.json_rpc("getheight", Value::Null))?;
                let address = wallet.json_rpc("get_address", json!({ "account_index": 0 }));
                let balance = wallet.json_rpc("get_balance", json!({ "account_index": 0 }));

                wallet_version = version
                    .get("result")
                    .and_then(|result| result.get("version"))
                    .map(render_json_value);
                wallet_height = height
                    .get("result")
                    .and_then(|result| result.get("height"))
                    .map(render_json_value);
                wallet_address = address
                    .ok()
                    .and_then(|value| value.get("result").cloned())
                    .and_then(|result| {
                        result
                            .get("address")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned)
                            .or_else(|| {
                                result
                                    .get("addresses")
                                    .and_then(Value::as_array)
                                    .and_then(|items| items.first())
                                    .and_then(|entry| entry.get("address"))
                                    .and_then(Value::as_str)
                                    .map(ToOwned::to_owned)
                            })
                    });
                wallet_balance = balance
                    .ok()
                    .and_then(|value| value.get("result").cloned())
                    .and_then(|result| {
                        result
                            .get("balance")
                            .map(format_wallet_balance)
                            .or_else(|| {
                                result
                                    .get("balances")
                                    .and_then(Value::as_array)
                                    .and_then(|balances| balances.first())
                                    .and_then(|entry| entry.get("balance"))
                                    .map(format_wallet_balance)
                            })
                    });
                wallet_status = "Connected".into();
                context.wallet_address = wallet_address.clone();

                wallet_json = json!({
                    "get_version": version,
                    "get_height": height,
                });

                if let Some(method_name) = wallet_method {
                    if let Some(spec) = find_method(wallet_inventory, method_name) {
                        let preset = presets_for_method(RpcKind::Wallet, spec, &context)
                            .into_iter()
                            .find(|preset| Some(preset.key.as_str()) == wallet_param)
                            .or_else(|| {
                                presets_for_method(RpcKind::Wallet, spec, &context)
                                    .into_iter()
                                    .next()
                            });
                        wallet_selected_output = Some(Self::poll_selected_method(
                            &bundle,
                            RpcKind::Wallet,
                            spec,
                            preset.as_ref(),
                            wallet_inputs,
                            &mut context,
                        )?);
                    }
                }
                Ok(())
            })() {
                Ok(()) => {}
                Err(error) if require_wallet => return Err(error),
                Err(error) => wallet_error = Some(error),
            }
        }

        let outcome = PollOutcome {
            daemon_polled: true,
            daemon_status,
            daemon_version,
            daemon_height,
            current_block_height,
            target_height,
            nettype,
            peer_count,
            daemon_selected_output,
            wallet_status,
            wallet_version,
            wallet_height,
            wallet_address,
            wallet_balance,
            wallet_selected_output,
            wallet_polled: settings.wallet_rpc_enabled,
            raw_json: json!({
                "current_block": current_block,
                "daemon": daemon_raw_json,
                "wallet": wallet_json,
            }),
            notice: Some(match (&wallet_error, settings.wallet_rpc_enabled) {
                (Some(_), true) => "Daemon connected. Wallet RPC check failed.".into(),
                (None, true) => "Daemon and wallet RPC checks completed.".into(),
                (_, false) => "Daemon check completed. Wallet RPC is disabled.".into(),
            }),
            error: wallet_error,
        };

        Ok((bundle, outcome))
    }

    fn poll_selected_method(
        bundle: &RpcBundle,
        kind: RpcKind,
        spec: &RpcMethodSpec,
        preset: Option<&ParamPreset>,
        inputs: &BTreeMap<String, String>,
        context: &mut RpcContext,
    ) -> Result<Value, String> {
        let payload = Self::payload_for_method(spec, preset, inputs)?;

        if !is_read_only_method(kind, &spec.method) {
            return Ok(json!({
                "method": spec.method,
                "command": spec.command,
                "template": preset.map(|preset| preset.label.clone()).unwrap_or_else(|| "empty request".into()),
                "payload": payload,
                "warning": "This RPC can mutate state and is not executed automatically from the monitor.",
            }));
        }

        match kind {
            RpcKind::Daemon => bundle.daemon.call(&spec.method, payload),
            RpcKind::Wallet => {
                let wallet = bundle
                    .wallet
                    .as_ref()
                    .ok_or_else(|| "Wallet RPC is disabled.".to_string())?;
                let value = wallet.call(&spec.method, payload)?;
                if context.wallet_address.is_none() {
                    context.wallet_address = value
                        .get("result")
                        .and_then(|result| result.get("address"))
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned);
                }
                Ok(value)
            }
        }
    }

    fn apply_poll(&mut self, bundle: RpcBundle, outcome: PollOutcome) {
        let polled_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        self.rpc = Some(bundle);
        self.daemon_status = outcome.daemon_status;
        self.wallet_status = outcome.wallet_status;
        self.daemon_version = outcome.daemon_version;
        self.daemon_height = outcome.daemon_height;
        self.current_block_height = outcome.current_block_height;
        self.target_height = outcome.target_height;
        self.nettype = outcome.nettype;
        self.peer_count = outcome.peer_count;
        self.wallet_version = outcome.wallet_version;
        self.wallet_height = outcome.wallet_height;
        self.wallet_address = outcome.wallet_address;
        self.wallet_balance = outcome.wallet_balance;
        self.last_rpc_json = Some(outcome.raw_json);
        if let Some(output) = outcome.daemon_selected_output {
            self.selected_daemon_output = Some(output);
        }
        if let Some(output) = outcome.wallet_selected_output {
            self.selected_wallet_output = Some(output);
        }
        self.notice = outcome.notice;
        self.error = outcome.error;
        if outcome.daemon_polled {
            self.last_daemon_poll = polled_at.clone();
        }
        if outcome.wallet_polled {
            self.last_wallet_poll = polled_at;
        }
    }

    fn selected_method_spec(&self, kind: RpcKind) -> Option<&RpcMethodSpec> {
        match kind {
            RpcKind::Daemon => self
                .daemon_method
                .as_deref()
                .and_then(|method| find_method(&self.daemon_inventory, method)),
            RpcKind::Wallet => self
                .wallet_method
                .as_deref()
                .and_then(|method| find_method(&self.wallet_inventory, method)),
        }
    }

    fn field_input_map(&self, kind: RpcKind) -> &BTreeMap<String, String> {
        match kind {
            RpcKind::Daemon => &self.daemon_field_inputs,
            RpcKind::Wallet => &self.wallet_field_inputs,
        }
    }

    fn refresh_request_inputs(&mut self, kind: RpcKind) {
        let Some(method) = self.selected_method_spec(kind).cloned() else {
            return;
        };

        let payload = self
            .selected_preset(kind)
            .map(|preset| preset.payload)
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

        let inputs = input_strings_from_payload(&method.request_fields, &payload);

        match kind {
            RpcKind::Daemon => self.daemon_field_inputs = inputs,
            RpcKind::Wallet => self.wallet_field_inputs = inputs,
        }
    }

    fn selected_preset(&self, kind: RpcKind) -> Option<ParamPreset> {
        let selected = match kind {
            RpcKind::Daemon => self.daemon_param.as_deref(),
            RpcKind::Wallet => self.wallet_param.as_deref(),
        };

        self.presets_for_selected_method(kind)
            .into_iter()
            .find(|preset| Some(preset.key.as_str()) == selected)
            .or_else(|| self.presets_for_selected_method(kind).into_iter().next())
    }

    fn payload_for_method(
        spec: &RpcMethodSpec,
        preset: Option<&ParamPreset>,
        inputs: &BTreeMap<String, String>,
    ) -> Result<Value, String> {
        if spec.request_fields.is_empty() {
            return Ok(Value::Null);
        }

        let preset_payload = preset
            .map(|preset| preset.payload.clone())
            .unwrap_or(Value::Null);
        let preset_inputs = input_strings_from_payload(&spec.request_fields, &preset_payload);
        let mut payload = serde_json::Map::new();

        for field in &spec.request_fields {
            let value = inputs
                .get(&field.name)
                .cloned()
                .or_else(|| preset_inputs.get(&field.name).cloned())
                .unwrap_or_default();
            payload.insert(field.name.clone(), parse_input_value(field, &value)?);
        }

        Ok(Value::Object(payload))
    }

    fn json_to_lines(&self, value: &Value, indent: usize) -> Vec<String> {
        let prefix = "  ".repeat(indent);

        match value {
            Value::Object(map) => map
                .iter()
                .flat_map(|(key, value)| match value {
                    Value::Object(_) | Value::Array(_) => {
                        let mut lines = vec![format!("{prefix}{key}:")];
                        lines.extend(self.json_to_lines(value, indent + 1));
                        lines
                    }
                    _ => vec![format!("{prefix}{key}: {}", render_json_value(value))],
                })
                .collect(),
            Value::Array(items) => items
                .iter()
                .flat_map(|item| match item {
                    Value::Object(_) | Value::Array(_) => {
                        let mut lines = vec![format!("{prefix}-")];
                        lines.extend(self.json_to_lines(item, indent + 1));
                        lines
                    }
                    _ => vec![format!("{prefix}- {}", render_json_value(item))],
                })
                .collect(),
            _ => vec![format!("{prefix}{}", render_json_value(value))],
        }
    }

    fn selected_daemon_output(&self) -> String {
        let Some(output) = &self.selected_daemon_output else {
            return "No daemon payload has been captured yet.".into();
        };
        self.json_to_lines(output, 0).join("\n")
    }

    fn selected_wallet_output(&self) -> String {
        if !self.wallet_rpc_enabled {
            return "Wallet RPC is disabled in settings.".into();
        }
        let Some(output) = &self.selected_wallet_output else {
            return "No wallet payload has been captured yet.".into();
        };
        self.json_to_lines(output, 0).join("\n")
    }
}

fn as_u64_string(value: &Value) -> Option<String> {
    match value {
        Value::Number(number) => Some(number.to_string()),
        Value::String(value) => Some(value.clone()),
        _ => None,
    }
}

fn validate_rpc_status(value: &Value, method: &str) -> Result<(), String> {
    if let Some(error) = value.get("error") {
        return Err(format!("{method} failed: {}", render_json_value(error)));
    }

    if let Some(status) = value
        .get("result")
        .and_then(|result| result.get("status"))
        .and_then(Value::as_str)
        .filter(|status| *status != "OK")
    {
        return Err(format!("{method} failed: {status}"));
    }

    Ok(())
}

fn render_json_value(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        _ => value.to_string(),
    }
}

fn format_wallet_balance(value: &Value) -> String {
    let raw = render_json_value(value);
    let negative = raw.starts_with('-');
    let digits = raw.trim_start_matches('-');

    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return raw;
    }

    let padded = if digits.len() <= 8 {
        format!("{digits:0>9}")
    } else {
        digits.to_string()
    };
    let split_at = padded.len() - 8;
    let whole = &padded[..split_at];
    let fractional = &padded[split_at..];
    let sign = if negative { "-" } else { "" };

    format!("{sign}{whole}.{fractional}")
}

fn panel_style(
    background: Color,
    text_color: Option<Color>,
    radius: Option<f32>,
) -> impl Fn(&Theme) -> container::Style {
    move |_theme: &Theme| {
        container::Style::default()
            .background(Background::Color(background))
            .color(text_color.unwrap_or(TEXT_MAIN))
            .border(
                Border::default()
                    .rounded(radius.unwrap_or(16.0))
                    .width(1.0)
                    .color(BORDER_SOFT),
            )
            .shadow(Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.18),
                offset: iced::Vector::new(0.0, 6.0),
                blur_radius: 18.0,
            })
    }
}

fn primary_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered => ACCENT,
        button::Status::Pressed => ACCENT_DIM,
        button::Status::Disabled => BG_PANEL,
        _ => ACCENT_DIM,
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color: TEXT_MAIN,
        border: Border::default().rounded(14.0),
        shadow: Shadow::default(),
        snap: false,
    }
}

fn top_button_style(active: bool, status: button::Status) -> button::Style {
    let background = if active {
        BG_PANEL_ALT
    } else {
        match status {
            button::Status::Hovered => BG_PANEL_ALT,
            button::Status::Pressed => BG_SIDEBAR,
            _ => Color::TRANSPARENT,
        }
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color: TEXT_MAIN,
        border: Border::default()
            .rounded(12.0)
            .width(1.0)
            .color(BORDER_SOFT),
        shadow: Shadow::default(),
        snap: false,
    }
}

fn sidebar_button_style(active: bool, status: button::Status) -> button::Style {
    let background = if active {
        BG_PANEL_ALT
    } else {
        match status {
            button::Status::Hovered => Color::from_rgb(0.12, 0.12, 0.13),
            button::Status::Pressed => BG_PANEL_ALT,
            _ => Color::TRANSPARENT,
        }
    };

    let border_color = if active { ACCENT_DIM } else { BORDER_SOFT };

    button::Style {
        background: Some(Background::Color(background)),
        text_color: TEXT_MAIN,
        border: Border::default()
            .rounded(14.0)
            .width(if active { 1.2 } else { 1.0 })
            .color(border_color),
        shadow: Shadow::default(),
        snap: false,
    }
}

fn input_style(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let border_color = match status {
        text_input::Status::Focused { .. } => ACCENT,
        text_input::Status::Hovered => ACCENT_DIM,
        _ => BORDER_SOFT,
    };

    text_input::Style {
        background: Background::Color(BG_PANEL),
        border: Border::default()
            .rounded(12.0)
            .width(1.0)
            .color(border_color),
        icon: TEXT_MUTED,
        placeholder: TEXT_MUTED,
        value: TEXT_MAIN,
        selection: ACCENT_DIM,
    }
}

fn transport_options() -> Vec<String> {
    vec!["http".to_string(), "https".to_string()]
}

fn normalize_transport(value: &str, fallback: &str) -> String {
    let lowered = value.trim().to_ascii_lowercase();
    match lowered.as_str() {
        "http" | "https" => lowered,
        _ => fallback.to_string(),
    }
}

fn daemon_pick_list_style(_theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let border_color = match status {
        pick_list::Status::Hovered | pick_list::Status::Opened { .. } => ACCENT_DIM,
        pick_list::Status::Active => BORDER_SOFT,
    };

    pick_list::Style {
        text_color: TEXT_MAIN,
        placeholder_color: TEXT_MUTED,
        handle_color: TEXT_MUTED,
        background: Background::Color(BG_PANEL_ALT),
        border: Border::default()
            .rounded(14.0)
            .width(1.0)
            .color(border_color),
    }
}

fn default_vertical_scroll_direction() -> iced::widget::scrollable::Direction {
    iced::widget::scrollable::Direction::Vertical(
        iced::widget::scrollable::Scrollbar::new()
            .width(12)
            .scroller_width(12)
            .margin(2),
    )
}

fn content_scrollable_style(
    _theme: &Theme,
    status: iced::widget::scrollable::Status,
) -> iced::widget::scrollable::Style {
    let scroller_color = match status {
        iced::widget::scrollable::Status::Dragged { .. } => ACCENT,
        iced::widget::scrollable::Status::Hovered { .. } => ACCENT_DIM,
        _ => Color::from_rgba(1.0, 1.0, 1.0, 0.22),
    };

    let rail = iced::widget::scrollable::Rail {
        background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05))),
        border: Border::default()
            .rounded(10.0)
            .width(1.0)
            .color(BORDER_SOFT),
        scroller: iced::widget::scrollable::Scroller {
            background: Background::Color(scroller_color),
            border: Border::default().rounded(10.0),
        },
    };

    iced::widget::scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.03))),
        auto_scroll: iced::widget::scrollable::default(&_theme.clone(), status).auto_scroll,
    }
}
