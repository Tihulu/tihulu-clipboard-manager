// SPDX-License-Identifier: GPL-3.0-or-later

use crate::clipboard;
use crate::config::Config;
use crate::fl;
use crate::model::ClipboardPayload;
use crate::storage::ClipboardStore;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::{window::Id, Alignment, Length, Limits, Subscription};
use cosmic::prelude::*;
use cosmic::widget;

#[derive(Default)]
pub struct AppModel {
    core: cosmic::Core,
    popup: Option<Id>,
    config: Config,
    store: ClipboardStore,
    confirm_clear_all: bool,
    backend_warning: Option<String>,
    last_action: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    ClipboardChanged(String),
    ClipboardBackendWarning(String),
    UpdateConfig(Config),
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

    fn init(core: cosmic::Core, _flags: Self::Flags) -> (Self, Task<cosmic::Action<Self::Message>>) {
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
            AppModel {
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
        self.core
            .applet
            .icon_button("edit-paste-symbolic")
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        let mut content = widget::list_column()
            .spacing(8)
            .padding(12)
            .add(
                widget::row::with_children(vec![
                    widget::text::title3(fl!("app-title")).into(),
                    widget::horizontal_space().into(),
                    widget::button(fl!("clear-all"))
                        .on_press(Message::RequestClearAll)
                        .into(),
                ])
                .align_y(Alignment::Center),
            );

        if self.config.private_mode {
            content = content.add(widget::text(fl!("private-mode-enabled")));
        }

        if self.config.encrypt_history {
            content = content.add(widget::text(fl!("history-encrypted")));
        }

        if let Some(warning) = &self.backend_warning {
            content = content.add(widget::text(format!("{} {warning}", fl!("backend-warning"))));
        }

        if let Some(action) = &self.last_action {
            content = content.add(widget::text(action.clone()));
        }

        if self.confirm_clear_all {
            content = content.add(
                widget::container(
                    widget::column::with_children(vec![
                        widget::text(fl!("clear-all-confirmation")).into(),
                        widget::row::with_children(vec![
                            widget::button(fl!("cancel"))
                                .on_press(Message::CancelClearAll)
                                .into(),
                            widget::button(fl!("erase-all"))
                                .on_press(Message::ConfirmClearAll)
                                .into(),
                        ])
                        .spacing(8)
                        .into(),
                    ])
                    .spacing(8),
                )
                .width(Length::Fill),
            );
        }

        if self.store.entries().is_empty() {
            content = content.add(widget::text(fl!("history-empty")));
        } else {
            for entry in self.store.entries() {
                let pin_label = if entry.pinned { fl!("unpin") } else { fl!("pin") };
                content = content.add(
                    widget::container(
                        widget::column::with_children(vec![
                            widget::text(entry.preview()).into(),
                            widget::row::with_children(vec![
                                widget::button(fl!("copy"))
                                    .on_press(Message::CopyEntry(entry.id))
                                    .into(),
                                widget::button(pin_label)
                                    .on_press(Message::TogglePin(entry.id))
                                    .into(),
                                widget::button(fl!("delete"))
                                    .on_press(Message::DeleteEntry(entry.id))
                                    .into(),
                            ])
                            .spacing(8)
                            .into(),
                        ])
                        .spacing(6),
                    )
                    .width(Length::Fill),
                );
            }
        }

        content = content.add(
            widget::button(fl!("clear-unpinned"))
                .on_press(Message::ClearUnpinned),
        );

        self.core.applet.popup_container(content).into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            Subscription::run(|| {
                cosmic::iced::stream::channel(32, clipboard::watch_text_clipboard)
            }),
            self.core().watch_config::<Config>(Self::APP_ID).map(|update| {
                Message::UpdateConfig(update.config)
            }),
        ])
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::ClipboardChanged(text) => {
                let result = self.store.add_text(text, &self.config);
                if matches!(result, crate::storage::AddTextResult::Added) {
                    self.backend_warning = None;
                    let _ = self.store.save(&self.config);
                }
            }
            Message::ClipboardBackendWarning(warning) => {
                self.backend_warning = Some(warning);
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
            Message::CopyEntry(id) => {
                if let Some(entry) = self.store.entries().iter().find(|entry| entry.id == id) {
                    let ClipboardPayload::Text(text) = &entry.payload;
                    let text = text.clone();
                    return Task::perform(
                        async move { clipboard::copy_text_to_clipboard(text).await },
                        |result| cosmic::Action::App(Message::EntryCopied(result)),
                    );
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
                let _ = ClipboardStore::delete_persisted_files();
                let _ = self.store.save(&self.config);
            }
            Message::CancelClearAll => {
                self.confirm_clear_all = false;
            }
            Message::ClearUnpinned => {
                self.store.clear_unpinned();
                let _ = self.store.save(&self.config);
            }
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );
                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(420.0)
                        .min_width(340.0)
                        .min_height(260.0)
                        .max_height(900.0);
                    get_popup(popup_settings)
                };
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
        }

        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}
