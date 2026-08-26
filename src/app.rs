// SPDX-License-Identifier: GPL-3.0-or-later

use crate::clipboard;
use crate::config::Config;
use crate::fl;
use crate::model::{ClipboardEntry, human_size};
use crate::storage::{AddContentResult, ClipboardStore};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::{Alignment, Length, Limits, Subscription, window::Id};
use cosmic::prelude::*;
use cosmic::widget;

const PREVIEW_MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum PopupKind {
    History,
    Settings,
}

#[derive(Default)]
pub struct AppModel {
    core: cosmic::Core,
    popup: Option<Id>,
    popup_kind: Option<PopupKind>,
    config: Config,
    store: ClipboardStore,
    confirm_clear_all: bool,
    last_action: Option<String>,
    search_query: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleHistoryPopup,
    ToggleSettingsPopup,
    PopupClosed(Id),
    ClipboardChanged(String),
    ClipboardImageChanged { mime: String, bytes: Box<[u8]> },
    UpdateConfig(Config),
    SearchChanged(String),
    ClearSearch,
    SetPrivateMode(bool),
    SetUniqueSession(bool),
    SetSensitiveFilter(bool),
    SetImageClipboard(bool),
    SetImageLimit(bool),
    CopyEntry(u64),
    EntryCopied(Result<(), String>),
    DeleteEntry(u64),
    TogglePin(u64),
    RequestClearAll,
    ConfirmClearAll,
    CancelClearAll,
    ClearUnpinned,
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "io.github.tihulu.ClipboardManager";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let config = cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
            .map(|context| match Config::get_entry(&context) {
                Ok(config) => config,
                Err((_errors, config)) => config,
            })
            .unwrap_or_default();

        if config.unique_session {
            let _ = ClipboardStore::delete_persisted_files();
        }

        let mut store = ClipboardStore::load_or_default(&config);
        store.prune(&config);
        let _ = store.save(&config);

        (
            Self {
                core,
                config,
                store,
                ..Default::default()
            },
            Task::none(),
        )
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn view(&self) -> Element<'_, Self::Message> {
        widget::mouse_area(
            self.core
                .applet
                .icon_button("edit-paste-symbolic")
                .on_press(Message::ToggleHistoryPopup),
        )
        .on_right_press(Message::ToggleSettingsPopup)
        .into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        match self.popup_kind.unwrap_or(PopupKind::History) {
            PopupKind::History => self.history_popup(),
            PopupKind::Settings => self.settings_popup(),
        }
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let mut subscriptions = vec![
            Subscription::run(|| cosmic::iced::stream::channel(32, clipboard::watch_text_clipboard)),
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| Message::UpdateConfig(update.config)),
        ];

        if self.config.image_clipboard {
            subscriptions.push(Subscription::run(|| {
                cosmic::iced::stream::channel(4, clipboard::watch_image_clipboard)
            }));
        }

        Subscription::batch(subscriptions)
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::ClipboardChanged(text) => {
                if matches!(
                    self.store.add_text(text, &self.config),
                    AddContentResult::Added
                ) {
                    let _ = self.store.save(&self.config);
                }
            }
            Message::ClipboardImageChanged { mime, bytes } => {
                if matches!(
                    self.store.add_image(mime, bytes.as_ref(), &self.config),
                    AddContentResult::Added
                ) {
                    let _ = self.store.save(&self.config);
                }
            }
            Message::UpdateConfig(config) => {
                let encryption_changed = self.config.encrypt_history != config.encrypt_history;
                let unique_session_enabled = !self.config.unique_session && config.unique_session;
                self.config = config;

                if unique_session_enabled {
                    self.store.clear_all();
                    let _ = ClipboardStore::delete_persisted_files();
                }

                self.store.prune(&self.config);

                if encryption_changed {
                    let _ = ClipboardStore::delete_persisted_files();
                }

                let _ = self.store.save(&self.config);
            }
            Message::SearchChanged(query) => {
                self.search_query = query;
            }
            Message::ClearSearch => {
                self.search_query.clear();
            }
            Message::SetPrivateMode(value) => {
                self.config.private_mode = value;
                self.last_action = Some(if value {
                    fl!("incognito-enabled")
                } else {
                    fl!("incognito-disabled")
                });
                persist_config(&self.config);
            }
            Message::SetUniqueSession(value) => {
                self.config.unique_session = value;
                self.last_action = Some(if value {
                    fl!("unique-session-enabled")
                } else {
                    fl!("unique-session-disabled")
                });
                persist_config(&self.config);
            }
            Message::SetSensitiveFilter(value) => {
                self.config.sensitive_filter = value;
                self.last_action = Some(if value {
                    fl!("sensitive-filter-enabled")
                } else {
                    fl!("sensitive-filter-disabled")
                });
                persist_config(&self.config);
            }
            Message::SetImageClipboard(value) => {
                self.config.image_clipboard = value;
                self.last_action = Some(if value {
                    fl!("image-clipboard-enabled")
                } else {
                    fl!("image-clipboard-disabled")
                });
                persist_config(&self.config);
            }
            Message::SetImageLimit(value) => {
                self.config.limit_image_size = value;
                self.last_action = Some(if value {
                    fl!("image-limit-enabled")
                } else {
                    fl!("image-limit-disabled")
                });
                persist_config(&self.config);
            }
            Message::CopyEntry(id) => {
                if let Some(entry) = self.store.entries().iter().find(|entry| entry.id == id) {
                    if let Some(text) = entry.text() {
                        let text = text.to_string();
                        return Task::perform(
                            async move { clipboard::copy_text_to_clipboard(text).await },
                            |result| cosmic::Action::App(Message::EntryCopied(result)),
                        );
                    }

                    if let Some((mime, bytes)) = entry.image() {
                        let mime = mime.to_string();
                        return Task::perform(
                            async move {
                                clipboard::copy_image_to_clipboard(mime, bytes.into_boxed_slice())
                                    .await
                            },
                            |result| cosmic::Action::App(Message::EntryCopied(result)),
                        );
                    }
                }
            }
            Message::EntryCopied(result) => {
                self.last_action = Some(match result {
                    Ok(()) => fl!("copied"),
                    Err(error) => format!("{} {error}", fl!("copy-failed")),
                });
            }
            Message::DeleteEntry(id) => {
                self.store.delete(id);
                let _ = self.store.save(&self.config);
            }
            Message::TogglePin(id) => {
                self.store.toggle_pin(id);
                self.store.prune(&self.config);
                let _ = self.store.save(&self.config);
            }
            Message::RequestClearAll => {
                if self.config.confirm_before_clear_all {
                    self.confirm_clear_all = true;
                } else {
                    self.store.clear_all();
                    let _ = ClipboardStore::delete_persisted_files();
                    let _ = self.store.save(&self.config);
                }
            }
            Message::ConfirmClearAll => {
                self.store.clear_all();
                self.confirm_clear_all = false;
                self.search_query.clear();
                let _ = ClipboardStore::delete_persisted_files();
                let _ = self.store.save(&self.config);
            }
            Message::CancelClearAll => self.confirm_clear_all = false,
            Message::ClearUnpinned => {
                self.store.clear_unpinned();
                let _ = self.store.save(&self.config);
            }
            Message::ToggleHistoryPopup => return self.toggle_popup(PopupKind::History),
            Message::ToggleSettingsPopup => return self.toggle_popup(PopupKind::Settings),
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                    self.popup_kind = None;
                    self.confirm_clear_all = false;
                }
            }
        }

        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

impl AppModel {
    fn toggle_popup(&mut self, kind: PopupKind) -> Task<cosmic::Action<Message>> {
        if self.popup_kind == Some(kind) {
            self.popup_kind = None;
            self.confirm_clear_all = false;

            if let Some(id) = self.popup.take() {
                return destroy_popup(id);
            }

            return Task::none();
        }

        let mut tasks = Vec::new();
        if let Some(id) = self.popup.take() {
            tasks.push(destroy_popup(id));
        }

        let id = Id::unique();
        self.popup = Some(id);
        self.popup_kind = Some(kind);
        self.confirm_clear_all = false;

        let mut settings = self.core.applet.get_popup_settings(
            self.core.main_window_id().unwrap(),
            id,
            None,
            None,
            None,
        );

        settings.positioner.size_limits = match kind {
            PopupKind::History => Limits::NONE
                .min_width(640.0)
                .max_width(780.0)
                .min_height(420.0)
                .max_height(820.0),
            PopupKind::Settings => Limits::NONE
                .min_width(340.0)
                .max_width(380.0)
                .min_height(260.0)
                .max_height(440.0),
        };

        tasks.push(get_popup(settings));
        Task::batch(tasks)
    }

    fn history_popup(&self) -> Element<'_, Message> {
        let query = self.search_query.trim().to_lowercase();
        let is_searching = !query.is_empty();
        let entries: Vec<&ClipboardEntry> = self
            .store
            .entries()
            .iter()
            .filter(|entry| !is_searching || entry_matches_query(entry, &query))
            .collect();

        let mut list = widget::column::with_capacity(entries.len().max(1))
            .spacing(12)
            .width(Length::Fill);

        if self.store.entries().is_empty() {
            list = list.push(widget::container(widget::text(fl!("history-empty"))).padding(12));
        } else if entries.is_empty() {
            list = list.push(widget::container(widget::text(fl!("no-results"))).padding(12));
        } else {
            for entry in entries.iter().copied() {
                list = list.push(entry_card(entry));
            }
        }

        let mut content = widget::column::with_capacity(9)
            .spacing(12)
            .padding(16)
            .push(header_row())
            .push(status_row(&self.config))
            .push(search_row(&self.search_query));

        if let Some(action) = &self.last_action {
            content = content.push(widget::container(widget::text(action.clone())).padding(8));
        }

        if self.confirm_clear_all {
            content = content.push(confirm_clear_box());
        }

        content = content
            .push(widget::divider::horizontal::light())
            .push(
                widget::scrollable(list)
                    .height(Length::Fixed(610.0))
                    .width(Length::Fill),
            )
            .push(widget::divider::horizontal::light())
            .push(
                widget::row::with_children(vec![
                    widget::button::text(fl!("clear-unpinned"))
                        .on_press(Message::ClearUnpinned)
                        .into(),
                    widget::Space::new().width(Length::Fill).into(),
                    widget::text(result_count_label(
                        is_searching,
                        entries.len(),
                        self.store.entries().len(),
                    ))
                    .into(),
                ])
                .align_y(Alignment::Center),
            );

        self.core.applet.popup_container(content).into()
    }

    fn settings_popup(&self) -> Element<'_, Message> {
        let mut content = widget::column::with_capacity(9)
            .spacing(14)
            .padding(14)
            .push(widget::text::title3(fl!("app-title")))
            .push(settings_switch_row(
                fl!("incognito"),
                self.config.private_mode,
                Message::SetPrivateMode,
            ))
            .push(settings_switch_row(
                fl!("unique-session"),
                self.config.unique_session,
                Message::SetUniqueSession,
            ))
            .push(settings_switch_row(
                fl!("sensitive-filter"),
                self.config.sensitive_filter,
                Message::SetSensitiveFilter,
            ))
            .push(settings_switch_row(
                fl!("image-history"),
                self.config.image_clipboard,
                Message::SetImageClipboard,
            ))
            .push(settings_switch_row(
                fl!("image-limit"),
                self.config.limit_image_size,
                Message::SetImageLimit,
            ))
            .push(widget::divider::horizontal::light())
            .push(widget::button::text(fl!("clear-all")).on_press(Message::RequestClearAll));

        if self.confirm_clear_all {
            content = content.push(confirm_clear_box());
        }

        self.core.applet.popup_container(content).into()
    }
}

fn header_row() -> Element<'static, Message> {
    widget::row::with_children(vec![
        widget::column::with_children(vec![
            widget::text::title3(fl!("app-title")).into(),
            widget::text(fl!("app-subtitle")).into(),
        ])
        .spacing(2)
        .into(),
        widget::Space::new().width(Length::Fill).into(),
        widget::button::text(fl!("clear-all"))
            .on_press(Message::RequestClearAll)
            .into(),
    ])
    .align_y(Alignment::Center)
    .into()
}

fn status_row(config: &Config) -> Element<'static, Message> {
    widget::column::with_children(vec![
        widget::row::with_children(vec![
            badge(fl!("badge-encrypted"), config.encrypt_history),
            badge(fl!("badge-images"), config.image_clipboard),
        ])
        .spacing(10)
        .into(),
        widget::row::with_children(vec![
            badge(fl!("badge-sensitive"), config.sensitive_filter),
            badge(fl!("badge-incognito"), config.private_mode),
        ])
        .spacing(10)
        .into(),
    ])
    .spacing(8)
    .into()
}

fn search_row(query: &str) -> Element<'_, Message> {
    widget::text_input::text_input(fl!("search-placeholder"), query)
        .on_input(Message::SearchChanged)
        .on_clear(Message::ClearSearch)
        .width(Length::Fill)
        .into()
}

fn settings_switch_row(
    label: String,
    value: bool,
    on_toggle: fn(bool) -> Message,
) -> Element<'static, Message> {
    widget::row::with_children(vec![
        widget::text(label).into(),
        widget::Space::new().width(Length::Fill).into(),
        widget::toggler(value).on_toggle(on_toggle).into(),
    ])
    .align_y(Alignment::Center)
    .into()
}

fn confirm_clear_box() -> Element<'static, Message> {
    widget::container(
        widget::column::with_children(vec![
            widget::text(fl!("clear-all-confirmation")).into(),
            widget::row::with_children(vec![
                widget::button::text(fl!("cancel"))
                    .on_press(Message::CancelClearAll)
                    .into(),
                widget::button::text(fl!("erase-all"))
                    .on_press(Message::ConfirmClearAll)
                    .into(),
            ])
            .spacing(8)
            .into(),
        ])
        .spacing(8),
    )
    .width(Length::Fill)
    .padding(10)
    .into()
}

fn entry_card(entry: &ClipboardEntry) -> Element<'_, Message> {
    let pin_label = if entry.pinned {
        fl!("unpin")
    } else {
        fl!("pin")
    };
    let actions = widget::row::with_children(vec![
        widget::button::text(fl!("copy"))
            .on_press(Message::CopyEntry(entry.id))
            .into(),
        widget::button::text(pin_label)
            .on_press(Message::TogglePin(entry.id))
            .into(),
        widget::button::text(fl!("delete"))
            .on_press(Message::DeleteEntry(entry.id))
            .into(),
    ])
    .spacing(14);

    let body: Element<'_, Message> = if let Some((mime, size_bytes)) = entry.image_info() {
        let preview: Element<'_, Message> = if size_bytes <= PREVIEW_MAX_BYTES {
            if let Some((_, bytes)) = entry.image() {
                widget::image(widget::image::Handle::from_bytes(bytes))
                    .width(Length::Fill)
                    .height(Length::Fixed(260.0))
                    .into()
            } else {
                widget::text(fl!("image-preview-unavailable")).into()
            }
        } else {
            widget::text(fl!("image-preview-too-large")).into()
        };

        widget::column::with_children(vec![
            widget::container(preview)
                .padding(6)
                .width(Length::Fill)
                .into(),
            widget::text(format!(
                "{} · {mime} · {}",
                fl!("image-entry"),
                human_size(size_bytes)
            ))
            .width(Length::Fill)
            .into(),
            actions.into(),
        ])
        .spacing(10)
        .width(Length::Fill)
        .into()
    } else {
        widget::column::with_children(vec![
            widget::text(entry.preview()).width(Length::Fill).into(),
            actions.into(),
        ])
        .spacing(8)
        .width(Length::Fill)
        .into()
    };

    widget::container(body)
        .width(Length::Fill)
        .padding(12)
        .into()
}

fn badge(label: String, active: bool) -> Element<'static, Message> {
    let marker = if active { "●" } else { "○" };
    widget::container(widget::text(format!("{marker} {label}")))
        .padding(6)
        .into()
}

fn entry_matches_query(entry: &ClipboardEntry, query: &str) -> bool {
    let haystack = if let Some(text) = entry.text() {
        text.to_lowercase()
    } else {
        entry.preview().to_lowercase()
    };

    haystack.contains(query)
}

fn result_count_label(is_searching: bool, shown: usize, total: usize) -> String {
    if is_searching {
        format!("{}: {shown}/{total}", fl!("results"))
    } else {
        format!("{}: {total}", fl!("entries"))
    }
}

fn persist_config(config: &Config) {
    if let Ok(context) =
        cosmic_config::Config::new(<AppModel as cosmic::Application>::APP_ID, Config::VERSION)
    {
        let _ = config.write_entry(&context);
    }
}
