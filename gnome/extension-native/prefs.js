// SPDX-License-Identifier: GPL-3.0-or-later

import Adw from 'gi://Adw';
import GLib from 'gi://GLib';
import Gtk from 'gi://Gtk';

import {ExtensionPreferences} from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';

const HELPER_NAME = 'tihulu-gnome-clipboard-helper';

function helperPath() {
    return GLib.build_filenamev([GLib.get_home_dir(), '.local', 'bin', HELPER_NAME]);
}

function decodeBytes(bytes) {
    return new TextDecoder('utf-8').decode(bytes);
}

function runHelper(args) {
    const [ok, stdout, stderr, status] = GLib.spawn_sync(
        null,
        [helperPath(), ...args],
        null,
        GLib.SpawnFlags.SEARCH_PATH,
        null
    );

    if (!ok || status !== 0) {
        const detail = stderr ? decodeBytes(stderr).trim() : `exit status ${status}`;
        throw new Error(detail || 'helper failed');
    }

    return JSON.parse(stdout ? decodeBytes(stdout).trim() : '{}');
}

function addSwitch(group, title, subtitle, config, key) {
    const row = new Adw.SwitchRow({
        title,
        subtitle,
        active: config[key] === true,
    });

    row.connect('notify::active', () => {
        runHelper(['set', key, row.active ? 'true' : 'false']);
    });

    group.add(row);
}

function addSpin(group, title, subtitle, config, key, lower, upper, step) {
    const adjustment = new Gtk.Adjustment({
        lower,
        upper,
        step_increment: step,
        page_increment: step * 10,
        value: Number(config[key] ?? lower),
    });
    const row = new Adw.SpinRow({
        title,
        subtitle,
        adjustment,
        numeric: true,
    });

    row.connect('notify::value', () => {
        runHelper(['set', key, `${Math.floor(row.value)}`]);
    });

    group.add(row);
}

export default class TihuluPreferences extends ExtensionPreferences {
    fillPreferencesWindow(window) {
        window.set_title('Tihulu Clipboard Manager');
        window.set_default_size(620, 720);

        let state;
        try {
            state = runHelper(['state']);
        } catch (error) {
            const page = new Adw.PreferencesPage();
            const group = new Adw.PreferencesGroup({title: 'Native helper missing'});
            group.add(new Adw.ActionRow({
                title: 'Run the GNOME quick install script again.',
                subtitle: error.message,
            }));
            page.add(group);
            window.add(page);
            return;
        }

        const config = state.config || {};
        const page = new Adw.PreferencesPage({title: 'General'});

        const privacy = new Adw.PreferencesGroup({title: 'Privacy and security'});
        addSwitch(privacy, 'Encrypted history', 'Store history with native helper encryption.', config, 'encryptHistory');
        addSwitch(privacy, 'Sensitive filter', 'Skip common password, token, and key-like copied text.', config, 'sensitiveFilter');
        addSwitch(privacy, 'Private mode', 'Temporarily stop saving new clipboard entries.', config, 'privateMode');
        addSwitch(privacy, 'Unique session', 'Clear history for this session when enabled.', config, 'uniqueSession');
        page.add(privacy);

        const images = new Adw.PreferencesGroup({title: 'Images'});
        addSwitch(images, 'Image clipboard history', 'Capture copied PNG, JPEG, WebP, and GIF data.', config, 'imageClipboard');
        addSwitch(images, 'Image size limit', 'Skip images larger than the configured limit.', config, 'limitImageSize');
        addSpin(images, 'Maximum image bytes', 'Default is 25 MiB.', config, 'maxImageBytes', 1024, 100 * 1024 * 1024, 1024 * 1024);
        page.add(images);

        const retention = new Adw.PreferencesGroup({title: 'Retention'});
        addSpin(retention, 'Maximum entries', 'Pinned entries are kept first.', config, 'maxEntries', 1, 500, 1);
        addSpin(retention, 'Maximum age in days', 'Set 0 to disable age pruning.', config, 'maxAgeDays', 0, 3650, 1);
        addSpin(retention, 'Maximum text bytes', 'Oversized text payloads are skipped.', config, 'maxTextBytes', 1, 10 * 1024 * 1024, 1024);
        page.add(retention);

        window.add(page);
    }
}
