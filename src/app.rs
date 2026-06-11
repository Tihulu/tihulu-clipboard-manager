// SPDX-License-Identifier: GPL-3.0-or-later

use crate::config::Config;
use crate::fl;
use crate::storage::ClipboardStore;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::{futures, window::Id, Alignment, Length, Limits, Subscription};
use cosmic::prelude::*;
use cosmic::widget;
use futures::SinkExt;

#[derive(Default)]
pub struct AppModel {
    core: cosmic::Core,
    popup: Option<Id>,
    config: Config,
    store: ClipboardStore,
    confirm_clear_all: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    ClipboardChanged(String),
    UpdateConfig(Config),
    CopyEntry(u64),
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

        let store = ClipboardStore::load_or_default();

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
                cosmic::iced::stream::channel(4, move |_channel: futures::channel::mpsc::Sender<_>| async move {
                    // TODO: Replace this placeholder with Wayland data-control clipboard events.
                    futures::future::pending().await
                })
            }),
            self.core().watch_config::<Config>(Self::APP_ID).map(|update| {
                Message::UpdateConfig(update.config)
            }),
        ])
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::ClipboardChanged(text) => {
                self.store.add_text(text);
                self.store.prune_to_max_entries(self.config.max_entries);
                let _ = self.store.save();
            }
            Message::UpdateConfig(config) => {
                self.config = config;
                self.store.prune_to_max_entries(self.config.max_entries);
                let _ = self.store.save();
            }
            Message::CopyEntry(_id) => {
                // TODO: Set the Wayland clipboard to this entry's payload.
            }
            Message::DeleteEntry(id) => {
                self.store.delete(id);
                let _ = self.store.save();
            }
            Message::TogglePin(id) => {
                self.store.toggle_pin(id);
                self.store.prune_to_max_entries(self.config.max_entries);
                let _ = self.store.save();
            }
            Message::RequestClearAll => {
                if self.config.confirm_before_clear_all {
                    self.confirm_clear_all = true;
                } else {
                    self.store.clear_all();
                    let _ = self.store.save();
                }
            }
            Message::ConfirmClearAll => {
                self.store.clear_all();
                self.confirm_clear_all = false;
                let _ = self.store.save();
            }
            Message::CancelClearAll => {
                self.confirm_clear_all = false;
            }
            Message::ClearUnpinned => {
                self.store.clear_unpinned();
                let _ = self.store.save();
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
