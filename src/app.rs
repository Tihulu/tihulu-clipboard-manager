// SPDX-License-Identifier: GPL-3.0-or-later

use crate::clipboard;
use crate::config::Config;
use crate::fl;
use crate::model::{ClipboardEntry, human_size};
use crate::storage::{AddContentResult, ClipboardStore};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::{window::Id, Alignment, Length, Limits, Subscription};
use cosmic::prelude::*;
use cosmic::widget;

const PREVIEW_MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Default)]
pub struct AppModel {
    core: cosmic::Core,
    popup: Option<Id>,
    config: Config,
    store: ClipboardStore,
    confirm_clear_all: bool,
    last_action: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    ClipboardChanged(String),
    ClipboardImageChanged { mime: String, bytes: Box<[u8]> },
    UpdateConfig(Config),
    TogglePrivateMode,
    ToggleUniqueSession,
    ToggleImageLimit,
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

    fn core(&self) -> &cosmic::Core { &self.core }

    fn core_mut(&mut self) -> &mut cosmic::Core { &mut self.core }

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

        (Self { core, config, store, ..Default::default() }, Task::none())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> { Some(Message::PopupClosed(id)) }

    fn view(&self) -> Element<'_, Self::Message> {
        self.core
            .applet
            .icon_button("edit-paste-symbolic")
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        let mut list = widget::column::with_capacity(self.store.entries().len().max(1))
            .spacing(8)
            .width(Length::Fill);

        if self.store.entries().is_empty() {
            list = list.push(widget::container(widget::text(fl!("history-empty"))).padding(12));
        } else {
            for entry in self.store.entries() {
                list = list.push(entry_card(entry));
            }
        }

        let mut content = widget::column::with_capacity(8)
            .spacing(10)
            .padding(12)
            .push(header_row())
            .push(status_row(&self.config))
            .push(toggle_row(&self.config));

        if let Some(action) = &self.last_action {
            content = content.push(widget::container(widget::text(action.clone())).padding(8));
        }

        if self.confirm_clear_all {
            content = content.push(confirm_clear_box());
        }

        content = content
            .push(widget::divider::horizontal::light())
            .push(list)
            .push(widget::divider::horizontal::light())
            .push(
                widget::row::with_children(vec![
                    widget::button::text(fl!("clear-unpinned"))
                        .on_press(Message::ClearUnpinned)
                        .into(),
                    widget::Space::new().width(Length::Fill).into(),
                    widget::text(format!("{}: {}", fl!("entries"), self.store.entries().len())).into(),
                ])
                .align_y(Alignment::Center),
            );

        self.core.applet.popup_container(content).into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            Subscription::run(|| cosmic::iced::stream::channel(32, clipboard::watch_clipboard)),
            self.core().watch_config::<Config>(Self::APP_ID).map(|update| Message::UpdateConfig(update.config)),
        ])
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::ClipboardChanged(text) => {
                if matches!(self.store.add_text(text, &self.config), AddContentResult::Added) {
                    let _ = self.store.save(&self.config);
                }
            }
            Message::ClipboardImageChanged { mime, bytes } => {
                if matches!(self.store.add_image(mime, bytes.as_ref(), &self.config), AddContentResult::Added) {
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
            Message::TogglePrivateMode => {
                self.config.private_mode = !self.config.private_mode;
                self.last_action = Some(if self.config.private_mode { fl!("incognito-enabled") } else { fl!("incognito-disabled") });
                persist_config(&self.config);
            }
            Message::ToggleUniqueSession => {
                self.config.unique_session = !self.config.unique_session;
                self.last_action = Some(if self.config.unique_session { fl!("unique-session-enabled") } else { fl!("unique-session-disabled") });
                persist_config(&self.config);
            }
            Message::ToggleImageLimit => {
                self.config.limit_image_size = !self.config.limit_image_size;
                self.last_action = Some(if self.config.limit_image_size { fl!("image-limit-enabled") } else { fl!("image-limit-disabled") });
                persist_config(&self.config);
            }
            Message::CopyEntry(id) => {
                if let Some(entry) = self.store.entries().iter().find(|entry| entry.id == id) {
                    if let Some(text) = entry.text() {
                        let text = text.to_string();
                        return Task::perform(async move { clipboard::copy_text_to_clipboard(text).await }, |result| cosmic::Action::App(Message::EntryCopied(result)));
                    }

                    if let Some((mime, bytes)) = entry.image() {
                        let mime = mime.to_string();
                        return Task::perform(async move { clipboard::copy_image_to_clipboard(mime, bytes.into_boxed_slice()).await }, |result| cosmic::Action::App(Message::EntryCopied(result)));
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
                let _ = ClipboardStore::delete_persisted_files();
                let _ = self.store.save(&self.config);
            }
            Message::CancelClearAll => self.confirm_clear_all = false,
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
                    let mut popup_settings = self.core.applet.get_popup_settings(self.core.main_window_id().unwrap(), new_id, None, None, None);
                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(460.0)
                        .min_width(380.0)
                        .min_height(320.0)
                        .max_height(960.0);
                    get_popup(popup_settings)
                };
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) { self.popup = None; }
            }
        }

        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> { Some(cosmic::applet::style()) }
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
        widget::button::text(fl!("clear-all")).on_press(Message::RequestClearAll).into(),
    ])
    .align_y(Alignment::Center)
    .into()
}

fn status_row(config: &Config) -> Element<'static, Message> {
    widget::row::with_children(vec![
        badge(fl!("badge-encrypted"), config.encrypt_history),
        badge(fl!("badge-images"), config.image_clipboard),
        badge(fl!("badge-incognito"), config.private_mode),
    ])
    .spacing(6)
    .into()
}

fn toggle_row(config: &Config) -> Element<'static, Message> {
    widget::row::with_children(vec![
        widget::button::text(toggle_label(fl!("incognito"), config.private_mode))
            .on_press(Message::TogglePrivateMode)
            .into(),
        widget::button::text(toggle_label(fl!("unique-session"), config.unique_session))
            .on_press(Message::ToggleUniqueSession)
            .into(),
        widget::button::text(image_limit_label(config))
            .on_press(Message::ToggleImageLimit)
            .into(),
    ])
    .spacing(8)
    .into()
}

fn confirm_clear_box() -> Element<'static, Message> {
    widget::container(
        widget::column::with_children(vec![
            widget::text(fl!("clear-all-confirmation")).into(),
            widget::row::with_children(vec![
                widget::button::text(fl!("cancel")).on_press(Message::CancelClearAll).into(),
                widget::button::text(fl!("erase-all")).on_press(Message::ConfirmClearAll).into(),
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
    let pin_label = if entry.pinned { fl!("unpin") } else { fl!("pin") };
    let actions = widget::row::with_children(vec![
        widget::button::text(fl!("copy")).on_press(Message::CopyEntry(entry.id)).into(),
        widget::button::text(pin_label).on_press(Message::TogglePin(entry.id)).into(),
        widget::button::text(fl!("delete")).on_press(Message::DeleteEntry(entry.id)).into(),
    ])
    .spacing(6);

    let body: Element<'_, Message> = if let Some((mime, size_bytes)) = entry.image_info() {
        let preview: Element<'_, Message> = if size_bytes <= PREVIEW_MAX_BYTES {
            if let Some((_, bytes)) = entry.image() {
                widget::image(widget::image::Handle::from_bytes(bytes))
                    .width(Length::Fixed(86.0))
                    .height(Length::Fixed(72.0))
                    .into()
            } else {
                widget::text(fl!("image-preview-unavailable")).into()
            }
        } else {
            widget::text(fl!("image-preview-too-large")).into()
        };

        widget::row::with_children(vec![
            widget::container(preview).padding(4).width(Length::Fixed(96.0)).into(),
            widget::column::with_children(vec![
                widget::text(format!("{} {mime}", fl!("image-entry"))).into(),
                widget::text(human_size(size_bytes)).into(),
                actions.into(),
            ])
            .spacing(5)
            .width(Length::Fill)
            .into(),
        ])
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
    } else {
        widget::column::with_children(vec![widget::text(entry.preview()).into(), actions.into()])
            .spacing(6)
            .into()
    };

    widget::container(body).width(Length::Fill).padding(10).into()
}

fn badge(label: String, active: bool) -> Element<'static, Message> {
    let marker = if active { "●" } else { "○" };
    widget::container(widget::text(format!("{marker} {label}"))).padding(6).into()
}

fn toggle_label(label: String, active: bool) -> String {
    if active { format!("{}: {}", label, fl!("on")) } else { format!("{}: {}", label, fl!("off")) }
}

fn image_limit_label(config: &Config) -> String {
    if config.limit_image_size {
        format!("{}: {}", fl!("image-limit"), fl!("limited-25-mib"))
    } else {
        format!("{}: {}", fl!("image-limit"), fl!("no-size-cap"))
    }
}

fn persist_config(config: &Config) {
    if let Ok(context) = cosmic_config::Config::new(
        <AppModel as cosmic::Application>::APP_ID,
        Config::VERSION,
    ) {
        let _ = config.write_entry(&context);
    }
}
