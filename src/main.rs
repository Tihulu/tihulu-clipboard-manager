// SPDX-License-Identifier: GPL-3.0-or-later

mod app;
mod config;
mod i18n;
mod model;
mod sensitive;
mod storage;

fn main() -> cosmic::iced::Result {
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
    i18n::init(&requested_languages);

    cosmic::applet::run::<app::AppModel>(())
}
