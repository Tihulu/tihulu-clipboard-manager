// SPDX-License-Identifier: GPL-3.0-or-later

mod app;
mod clipboard;
mod config;
mod i18n;
mod model;
mod sensitive;
mod single_instance;
mod storage;

fn main() -> cosmic::iced::Result {
    let _single_instance_guard = match single_instance::SingleInstanceGuard::acquire() {
        Ok(Some(guard)) => guard,
        Ok(None) => return Ok(()),
        Err(error) => {
            eprintln!("single instance lock warning: {error}");
            let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
            i18n::init(&requested_languages);
            return cosmic::applet::run::<app::AppModel>(());
        }
    };

    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
    i18n::init(&requested_languages);

    cosmic::applet::run::<app::AppModel>(())
}
